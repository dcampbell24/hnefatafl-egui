mod ai;
mod board;
mod game_play_view;
mod game_setup_view;

use crate::game_play_view::{GamePlayAction, GamePlayView};
use crate::game_setup_view::{GameSetupAction, GameSetupView};
use eframe::{App, Frame};
use egui::Context;
use hnefatafl::board::state::{BoardState, LargeBasicBoardState};
use std::process::exit;

enum View {
    GameSetup(GameSetupView),
    Game(GamePlayView<LargeBasicBoardState>)
}

struct MyApp {
    current_view: View,
}

impl App for MyApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let new_view = match self.current_view {
            View::GameSetup(ref mut game_setup_view) => {
                // Game setup screen
                match game_setup_view.update(ctx, frame) {
                    Some(GameSetupAction::StartGame(gs)) => Some(View::Game(GamePlayView::new(ctx, gs))),
                    Some(GameSetupAction::Quit) => exit(0),
                    _ => None
                }
            },
            View::Game(ref mut game_play_view) => {
                // Game play screen
                match game_play_view.update(ctx, frame) {
                    Some(GamePlayAction::QuitGame) => Some(View::GameSetup(GameSetupView::default())),
                    Some(GamePlayAction::QuitApp) => exit(0),
                    _ => None
                }
            }
        };
        if let Some(view) = new_view {
            self.current_view = view;
        }
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Tafl egui demo",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp {
                current_view: View::GameSetup(GameSetupView::default()),
            }))})
    ).unwrap();
}
