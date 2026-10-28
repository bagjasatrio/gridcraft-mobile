# gridcraft-android-spike

Android build and feasibility spike for GridCraft. Runs the real `SheetApp` in a single eframe viewport
with engine calculation, interactive grid, charts, touch gestures, mobile insets, and storage persistence.

## Features & Implementation

1. **Native GPU Rendering (Vulkan/wgpu)**:
   - Direct immediate-mode rendering of spreadsheet, conditional formatting, sparklines, and charts without webview overhead.
2. **Dynamic System Bar & IME Inset Handling**:
   - Status bar, navigation bar, and soft keyboard (IME) insets are queried dynamically via JNI (`WindowInsets.Type.systemBars() | WindowInsets.Type.ime()`).
   - The viewport automatically reserves bottom space for Gboard so the spreadsheet grid is never occluded.
   - Active cell auto-scrolls into the visible area (`ensure_visible = true`).
3. **Adaptive Rotation & Multi-Orientation**:
   - `screenOrientation = "unspecified"` enables free rotation between Landscape and Portrait.
   - Portrait mode provides 41 full visible grid rows (and 27 full rows with keyboard open).
   - Mobile toolbar adapts between compact icon-only (`⌨`, `−`, `+`, `💾`, `📂 Open`) and full text buttons based on screen width.
4. **Touch Navigation & Zoom**:
   - Single-finger tap to select cells, drag to scroll sheet.
   - Pinch gesture (`zoom_delta`) and mobile toolbar quick buttons (`[🔍 −]` / `[🔍 +]`) hooked to engine `view.zoom`.
5. **Storage Persistence & Open Dialog**:
   - Saving: `[💾 Save]` triggers `file.saveBytes` and writes valid `.xlsx` directly to Android app storage (`/sdcard/Android/data/ai.storyteller.gridcraft.spike/files/`).
   - Opening: `[📂 Samples / Open]` modal lists built-in templates (Sales, Budget, Grades, Blank) and files saved in device storage, opening them through `Services::inbox`.
6. **Soft Keyboard (IME)**:
   - `[⌨ Keyboard]` toolbar action calls `AndroidApp::show_soft_input(false)` via `android-activity` runtime safely without NDK pointer crashes.
7. **Optimized Release Packaging**:
   - Stripped binary with zipalign and keystore signing. APK size reduced from **220 MB** to **28 MB** (universal `aarch64` + `x86_64`).

## Prerequisites (Windows, as proven)

- Rust targets: `rustup target add aarch64-linux-android x86_64-linux-android`
- Android SDK + NDK, e.g. `~/AppData/Local/Android/Sdk` with `ndk/26.1.10909125`
- `cargo apk` (patched fork, as eframe's own `hello_android` README says):
  `cargo install --git https://github.com/parasyte/cargo-apk.git --rev 282639508eeed7d73f2e1eaeea042da2716436d5 cargo-apk`

## Build, install, run

```sh
export ANDROID_HOME="C:/Users/<you>/AppData/Local/Android/Sdk"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/26.1.10909125"
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
export PATH="$PATH:$ANDROID_HOME/build-tools/35.0.0:$ANDROID_HOME/platform-tools"

# Release build (28 MB signed APK):
cargo apk build --release -p gridcraft-android-spike --lib
adb install -r target/release/apk/gridcraft-android-spike.apk
adb shell am start -n ai.storyteller.gridcraft.spike/android.app.NativeActivity
```

The desktop build of the same code runs with `cargo run -p gridcraft-android-spike`.

## What works (verified on emulator Medium_Phone_API_35, x86_64)

- Compiles clean for `aarch64-linux-android` and `x86_64-linux-android`; **no desktop-only dependency
  blocks Android** (`rfd`, `arboard`, `craft-print`, `gridcraft-mcp` all live in `apps/gridcraft`, not
  in `ui-egui`).
- Renders through eframe/wgpu (Vulkan): ribbon, title bar, formula bar, grid, conditional formats,
  sparklines and charts all draw.
- Touch reaches egui: a tap moves the active cell, a swipe scrolls the grid (winit translates touch to
  pointer events; no extra work needed for basic navigation).
- System-bar insets are readable over JNI (`WindowInsets.Type.systemBars()`); the window reserves them
  with two `egui::Panel`s. Without that the status bar overlaps the app's title bar, because winit's
  Android backend hands over the whole surface (winit#3910).

## What does not work yet (the real cost of an Android port)

1. **Layout is the desktop layout.** The ribbon eats ~40% of a phone screen in landscape; ribbon groups
   are cut off at the right edge (`Cond Forma…`). Needs a compact/touch ribbon, not a restyle.
2. **Charts are clipped** by the viewport edge in the sample workbook.
3. **No gestures beyond tap/drag-scroll**: no pinch-zoom, no long-press context menu, no fill-handle
   drag, no two-finger scroll.
4. **No IME handling** — the cell editor's virtual keyboard was not tested.
5. **No file access**: opening/saving XLSX needs the Storage Access Framework (or `MANAGE_EXTERNAL_STORAGE`
   plus a picker); `Services::pick_open`/`pick_save` are `None` here.
6. **No lifecycle handling**: backgrounding/rotation is untested; winit recreates the surface and egui
   state must survive it.
7. **Debug APK is 220 MB** (x86_64 + arm64, debug info). A release build with `strip` + one ABI is the
   packaging starting point.

`docs/gaps.md` #23 estimates 30–60 h for a mobile layout; adding insets, gestures, IME, SAF file access
and packaging puts a usable Android build closer to 80–150 h. iOS is a separate, larger problem: eframe
has no iOS backend, so it needs a winit iOS shell (Swift/objc2) or the WASM build in a webview.

## Alternative that avoids the port

`apps/gridcraft-web` already builds the same UI to WASM (`trunk build --release`). Wrapping that build in
a Tauri v2 mobile shell (system webview + Rust backend) is far cheaper than a native port, at the cost of
WASM performance and webview input quirks. Tauri cannot host the native egui UI — it needs a web frontend.
