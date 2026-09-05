#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod presets;

use eframe::egui;

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_title("Binaural Beats")
        .with_app_id("binaural-beats")
        .with_inner_size([440.0, 820.0])
        .with_min_inner_size([380.0, 480.0]);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Binaural Beats",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
