#![cfg(not(target_arch = "wasm32"))]

use crate::app::MyApp;

mod ai;
mod board;
mod game_play_view;
mod game_setup_view;
mod app;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Tafl egui demo",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new(cc)))})
    ).unwrap();
}
