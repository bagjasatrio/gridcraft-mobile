//! Native Android implementation for GridCraft.
//!
//! Provides:
//! - Full SheetApp with engine & egui/wgpu rendering.
//! - System-bar insets (status bar & gesture navigation bar) via JNI.
//! - Soft input / virtual keyboard (IME) control via JNI with manual toggle & auto-trigger.
//! - File persistence: Save workbook to device storage and Open sample/saved workbooks via Inbox.
//! - Touch gestures: Pinch-to-zoom on touch screens & quick zoom buttons.
//!
//! Build: `cargo apk build --release -p gridcraft-android-spike --lib`

use std::sync::{Arc, Mutex};
use eframe::egui;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{Inbox, Services, SheetApp};
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct WindowInsetsState {
    pub top: f32,
    pub bottom: f32,
    pub ime_visible: bool,
}

pub struct SpikeApp {
    app: SheetApp,
    inbox: Inbox,
    /// System-bar and keyboard (IME) insets in physical pixels.
    insets: WindowInsetsState,
    #[cfg(target_os = "android")]
    android_app: Option<winit::platform::android::activity::AndroidApp>,
    #[cfg(target_os = "android")]
    android_app_handle: Arc<Mutex<Option<winit::platform::android::activity::AndroidApp>>>,
    show_samples_menu: bool,
    last_saved_path: Option<String>,
    was_editing: bool,
}

impl SpikeApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut session = Session::new();
        if let Err(e) = session.execute("file.new", json!({ "sample": "sales" })) {
            log::warn!("sample sales: {e}");
        }
        if session.documents().is_empty() {
            session.new_workbook();
        }

        let inbox: Inbox = Arc::new(Mutex::new(Vec::new()));
        let mem_clip = Arc::new(Mutex::new(String::new()));
        let mem_clip_copy = mem_clip.clone();
        let mem_clip_paste = mem_clip;

        #[cfg(target_os = "android")]
        let android_app_handle: Arc<Mutex<Option<winit::platform::android::activity::AndroidApp>>> = Arc::new(Mutex::new(None));
        #[cfg(target_os = "android")]
        let copy_app = android_app_handle.clone();
        #[cfg(target_os = "android")]
        let paste_app = android_app_handle.clone();

        let services = Services {
            inbox: Some(inbox.clone()),
            copy_html: Some(Box::new(move |_html: &str, text: &str| {
                *mem_clip_copy.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = text.to_string();
                #[cfg(target_os = "android")]
                if let Some(app) = &*copy_app.lock().unwrap_or_else(std::sync::PoisonError::into_inner) {
                    if let Err(e) = android_clipboard::set_clipboard_text(app, text) {
                        log::warn!("clipboard copy jni error: {e}");
                    }
                }
                Ok(())
            })),
            paste_text: Some(Box::new(move || {
                #[cfg(target_os = "android")]
                if let Some(app) = &*paste_app.lock().unwrap_or_else(std::sync::PoisonError::into_inner) {
                    if let Some(text) = android_clipboard::get_clipboard_text(app) {
                        if !text.is_empty() {
                            return Some(text);
                        }
                    }
                }
                let fallback = mem_clip_paste.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
                if fallback.is_empty() { None } else { Some(fallback) }
            })),
            download: Some(Box::new(|name: &str, bytes: &[u8]| {
                let candidates = [
                    format!("/sdcard/Android/data/ai.storyteller.gridcraft.spike/files/{name}"),
                    format!("/sdcard/Download/{name}"),
                    format!("/data/user/0/ai.storyteller.gridcraft.spike/files/{name}"),
                ];
                let mut saved = false;
                for p in &candidates {
                    if let Some(parent) = std::path::Path::new(p).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::write(p, bytes).is_ok() {
                        log::info!("workbook successfully saved: {p} ({} bytes)", bytes.len());
                        saved = true;
                        break;
                    }
                }
                if !saved {
                    log::warn!("could not write workbook to storage candidates");
                }
            })),
            ..Default::default()
        };

        let mut app = SheetApp::new(session, services);
        app.chrome.title_bar = false;
        app.ui.ribbon_collapsed = true;
        app.after_engine();
        SheetApp::setup_context_for_language(&cc.egui_ctx, app.ui.dark, app.ui.language);

        Self {
            app,
            inbox,
            insets: WindowInsetsState::default(),
            #[cfg(target_os = "android")]
            android_app: None,
            #[cfg(target_os = "android")]
            android_app_handle,
            show_samples_menu: false,
            last_saved_path: None,
            was_editing: false,
        }
    }

    #[cfg(target_os = "android")]
    fn with_android_app(mut self, app: winit::platform::android::activity::AndroidApp) -> Self {
        self.insets = android_insets::query_insets(&app);
        log::info!("system bar insets: top {} px, bottom {} px, ime={}", self.insets.top, self.insets.bottom, self.insets.ime_visible);
        *self.android_app_handle.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(app.clone());
        self.android_app = Some(app);
        self
    }

    /// Reserves the status and navigation bars out of the window before the app lays out.
    fn inset_panels(&mut self, ui: &mut egui::Ui) {
        #[cfg(target_os = "android")]
        if let Some(app) = &self.android_app {
            let current = android_insets::query_insets(app);
            if current != self.insets {
                log::info!(
                    "insets updated: top={:.1}px, bottom={:.1}px, ime={}",
                    current.top,
                    current.bottom,
                    current.ime_visible
                );
                self.insets = current;
                self.app.grid.ensure_visible = true;
                ui.ctx().request_repaint();
            }
        }
        let ppp = ui.ctx().pixels_per_point();
        let (top, bottom) = (self.insets.top / ppp, self.insets.bottom / ppp);
        if top > 0.0 {
            egui::Panel::top("android_status_bar_space").show(ui, |ui| ui.set_height(top));
        }
        if bottom > 0.0 {
            egui::Panel::bottom("android_nav_bar_space").show(ui, |ui| ui.set_height(bottom));
        }

        // Space optimization when virtual keyboard is open in landscape:
        let is_landscape = ui.available_width() > ui.available_height();
        self.app.ui.status_bar = !(self.insets.ime_visible && is_landscape);
    }

    /// Compact mobile quick action bar: Keyboard/Edit toggle, Copy, Paste, Zoom, Open Samples, Save.
    fn mobile_toolbar(&mut self, ui: &mut egui::Ui) {
        let compact = ui.available_width() < 550.0;
        let is_editing = self.app.editor.is_some();
        let active_cell_a1 = self
            .app
            .session
            .active()
            .map(|d| d.selection.active.a1())
            .unwrap_or_else(|| "A1".to_string());

        egui::Panel::top("android_mobile_toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = true;

                // 1. Edit mode controls
                if is_editing {
                    let commit_btn = egui::Button::new(egui::RichText::new("✓ Done").color(egui::Color32::from_rgb(30, 160, 30)).strong());
                    if ui.add(commit_btn).clicked() {
                        self.app.commit_edit(0, 0, false, false);
                        ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                    }
                    let cancel_btn = egui::Button::new(egui::RichText::new("✗ Cancel").color(egui::Color32::from_rgb(200, 40, 40)));
                    if ui.add(cancel_btn).clicked() {
                        self.app.cancel_edit();
                        ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                    }
                } else {
                    let edit_btn = if compact {
                        format!("⌨ {active_cell_a1}")
                    } else {
                        format!("⌨ Edit {active_cell_a1}")
                    };
                    if ui.button(edit_btn).clicked() {
                        self.app.begin_edit(None, false);
                        if let Some(ed) = self.app.editor.as_mut() {
                            ed.request_focus = true;
                        }
                        ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                        #[cfg(target_os = "android")]
                        if let Some(app) = &self.android_app {
                            android_ime::toggle_keyboard(app);
                        }
                    }
                }

                ui.separator();

                // 2. Clipboard controls (Copy & Paste)
                let copy_label = if compact { "📋" } else { "📋 Copy" };
                if ui.button(copy_label).clicked() {
                    self.app.copy_to_clipboard(ui.ctx(), "edit.copy");
                    ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                }

                let paste_label = if compact { "📥" } else { "📥 Paste" };
                if ui.button(paste_label).clicked() {
                    self.app.paste_from_clipboard();
                    ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                }

                ui.separator();

                // 3. Zoom controls
                let zoom_out = if compact { "−" } else { "🔍 −" };
                let zoom_in = if compact { "+" } else { "🔍 +" };
                if ui.button(zoom_out).clicked() {
                    self.adjust_zoom(0.85);
                    ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                }
                if ui.button(zoom_in).clicked() {
                    self.adjust_zoom(1.15);
                    ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                }

                // 4. Save workbook button
                let save_label = if compact { "💾" } else { "💾 Save" };
                if ui.button(save_label).clicked() {
                    self.save_current_workbook();
                    ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                }

                // 5. Samples / Open menu
                let open_label = if compact { "📂" } else { "📂 Open" };
                if ui.button(open_label).clicked() {
                    self.show_samples_menu = !self.show_samples_menu;
                    ui.ctx().memory_mut(|m| m.surrender_focus(ui.scope_id()));
                }

                if let Some(path) = &self.last_saved_path {
                    let text = if compact { "✓".to_string() } else { format!("✓ {path}") };
                    ui.label(egui::RichText::new(text).color(egui::Color32::GREEN).size(11.0));
                }
            });
        });
    }

    fn adjust_zoom(&mut self, factor: f32) {
        let current = self
            .app
            .session
            .active()
            .and_then(|d| d.wb.active().map(|s| s.zoom))
            .unwrap_or(100) as f32;
        let target = (current * factor).clamp(10.0, 400.0).round() as u16;
        let _ = self.app.session.run("view.zoom", json!({ "percent": target }));
        self.app.after_engine();
    }

    fn save_current_workbook(&mut self) {
        if let Ok(r) = self.app.run("file.saveBytes", json!({"format": "xlsx"}))
            && let Some(b64) = r.get("base64").and_then(serde_json::Value::as_str)
            && let Some(bytes) = gridcraft_engine::io::base64_decode(b64)
        {
            let name = r.get("name").and_then(serde_json::Value::as_str).unwrap_or("Book.xlsx");
            let target_path = format!("/sdcard/Android/data/ai.storyteller.gridcraft.spike/files/{name}");
            if let Some(parent) = std::path::Path::new(&target_path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(&target_path, &bytes).is_ok() {
                self.last_saved_path = Some(name.to_string());
                log::info!("Saved workbook to {target_path}");
            }
        }
    }

    fn samples_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_samples_menu {
            return;
        }
        egui::Window::new("Open Workbook / Sample")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("Select a Sample or File");
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui.button("📊 Sales Dashboard").clicked() {
                        let _ = self.app.session.execute("file.new", json!({"sample": "sales"}));
                        self.app.after_engine();
                        self.show_samples_menu = false;
                    }
                    if ui.button("💰 Budget & Expenses").clicked() {
                        let _ = self.app.session.execute("file.new", json!({"sample": "budget"}));
                        self.app.after_engine();
                        self.show_samples_menu = false;
                    }
                    if ui.button("🎓 Student Grades").clicked() {
                        let _ = self.app.session.execute("file.new", json!({"sample": "grades"}));
                        self.app.after_engine();
                        self.show_samples_menu = false;
                    }
                    if ui.button("📄 New Blank").clicked() {
                        self.app.session.new_workbook();
                        self.app.after_engine();
                        self.show_samples_menu = false;
                    }
                });

                ui.separator();
                ui.label("Saved in App Documents:");
                let dir = "/sdcard/Android/data/ai.storyteller.gridcraft.spike/files";
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().is_some_and(|ext| ext == "xlsx") {
                            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file.xlsx");
                            if ui.button(format!("📑 {file_name}")).clicked() {
                                if let Ok(bytes) = std::fs::read(&path) {
                                    self.inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((file_name.to_string(), bytes));
                                }
                                self.show_samples_menu = false;
                            }
                        }
                    }
                }

                ui.separator();
                if ui.button("Close").clicked() {
                    self.show_samples_menu = false;
                }
            });
    }

    /// Handles pinch gesture for sheet zoom and auto-triggers soft input on edit.
    fn handle_gestures(&mut self, ui: &egui::Ui) {
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > 0.02 {
            self.adjust_zoom(zoom_delta);
        }

        // Auto-show soft keyboard when editor transitions to active
        let is_editing = self.app.editor.is_some();
        if is_editing && !self.was_editing {
            #[cfg(target_os = "android")]
            if let Some(app) = &self.android_app {
                android_ime::toggle_keyboard(app);
            }
        }
        self.was_editing = is_editing;

        // Auto-scroll active cell into visible area while editing or when keyboard is shown
        if is_editing || self.insets.ime_visible {
            self.app.grid.ensure_visible = true;
        }
    }
}

impl eframe::App for SpikeApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.logic(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.inset_panels(ui);
        self.mobile_toolbar(ui);
        self.handle_gestures(ui);
        self.samples_dialog(ui.ctx());
        self.app.ui(ui);
    }
}

/// The system bars & keyboard (IME) insets, read from the running activity over JNI.
#[cfg(target_os = "android")]
mod android_insets {
    use jni::objects::{JObject, JValue};
    use winit::platform::android::activity::AndroidApp;
    use super::WindowInsetsState;

    pub fn query_insets(app: &AndroidApp) -> WindowInsetsState {
        unsafe { query(app) }.unwrap_or_else(|e| {
            log::warn!("system bar insets unavailable: {e}");
            WindowInsetsState::default()
        })
    }

    unsafe fn query(app: &AndroidApp) -> Result<WindowInsetsState, jni::errors::Error> {
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        let decor = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
        let insets = env
            .call_method(&decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?
            .l()?;
        if insets.is_null() {
            return Ok(WindowInsetsState::default());
        }
        let system_bars = env
            .call_static_method("android/view/WindowInsets$Type", "systemBars", "()I", &[])
            .and_then(|v| v.i())
            .unwrap_or(7);
        let ime = env
            .call_static_method("android/view/WindowInsets$Type", "ime", "()I", &[])
            .and_then(|v| v.i())
            .unwrap_or(8);
        let ime_visible = env
            .call_method(&insets, "isVisible", "(I)Z", &[JValue::Int(ime)])
            .and_then(|v| v.z())
            .unwrap_or(false);
        let types = system_bars | ime;
        let bars = env
            .call_method(&insets, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(types)])?
            .l()?;
        let top = env.get_field(&bars, "top", "I")?.i()? as f32;
        let bottom = env.get_field(&bars, "bottom", "I")?.i()? as f32;
        Ok(WindowInsetsState {
            top,
            bottom,
            ime_visible,
        })
    }
}

/// Soft keyboard (IME) control via AndroidApp.
#[cfg(target_os = "android")]
mod android_ime {
    use winit::platform::android::activity::AndroidApp;

    pub fn toggle_keyboard(app: &AndroidApp) {
        log::info!("requesting AndroidApp::show_soft_input(false)");
        app.show_soft_input(false);
    }
}

/// System clipboard bridge via Android ClipboardManager JNI.
#[cfg(target_os = "android")]
mod android_clipboard {
    use jni::objects::{JObject, JValue};
    use winit::platform::android::activity::AndroidApp;

    pub fn set_clipboard_text(app: &AndroidApp, text: &str) -> Result<(), jni::errors::Error> {
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let service_name = env.new_string("clipboard")?;
        let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&service_name)])?.l()?;
        if cm.is_null() {
            return Ok(());
        }
        let label = env.new_string("GridCraft")?;
        let text_jstr = env.new_string(text)?;
        let clip_data = env.call_static_method(
            "android/content/ClipData",
            "newPlainText",
            "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
            &[JValue::Object(&label), JValue::Object(&text_jstr)],
        )?.l()?;
        env.call_method(&cm, "setPrimaryClip", "(Landroid/content/ClipData;)V", &[JValue::Object(&clip_data)])?;
        Ok(())
    }

    pub fn get_clipboard_text(app: &AndroidApp) -> Option<String> {
        unsafe { get(app) }.unwrap_or_default()
    }

    unsafe fn get(app: &AndroidApp) -> Result<Option<String>, jni::errors::Error> {
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let service_name = env.new_string("clipboard")?;
        let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&service_name)])?.l()?;
        if cm.is_null() {
            return Ok(None);
        }
        let has_clip = env.call_method(&cm, "hasPrimaryClip", "()Z", &[])?.z()?;
        if !has_clip {
            return Ok(None);
        }
        let clip = env.call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[])?.l()?;
        if clip.is_null() {
            return Ok(None);
        }
        let count = env.call_method(&clip, "getItemCount", "()I", &[])?.i()?;
        if count <= 0 {
            return Ok(None);
        }
        let item = env.call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[JValue::Int(0)])?.l()?;
        let char_seq = env.call_method(&item, "coerceToText", "(Landroid/content/Context;)Ljava/lang/CharSequence;", &[JValue::Object(&activity)])?.l()?;
        let jstr = env.call_method(&char_seq, "toString", "()Ljava/lang/String;", &[])?.l()?;
        let rust_str: String = env.get_string(&jstr.into())?.into();
        Ok(Some(rust_str))
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info));
    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        ..Default::default()
    };
    let spike = std::cell::RefCell::new(Some(app));
    let run = eframe::run_native(
        "GridCraft",
        options,
        Box::new(move |cc| {
            let mut app = SpikeApp::new(cc);
            if let Some(android_app) = spike.borrow_mut().take() {
                app = app.with_android_app(android_app);
            }
            Ok(Box::new(app))
        }),
    );
    if let Err(e) = run {
        log::error!("gridcraft android spike: {e}");
    }
}
