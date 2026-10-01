#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod presets;

use eframe::egui;

fn main() -> eframe::Result {
    // Lets CI smoke-test release binaries without opening a window.
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("OpenBinauralBeats {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("OpenBinauralBeats")
        .with_app_id("open-binaural-beats")
        .with_inner_size([440.0, 820.0])
        .with_min_inner_size([380.0, 480.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "OpenBinauralBeats",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
