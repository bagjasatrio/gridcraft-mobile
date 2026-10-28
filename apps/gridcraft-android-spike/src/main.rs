//! Desktop entry point for the Android spike, so the same code can be looked at on the host.

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "GridCraft Spike",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(gridcraft_android_spike::SpikeApp::new(cc)))),
    )
}
