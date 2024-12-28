use crate::game_play_view::{GamePlayAction, GamePlayView};
use crate::game_setup_view::{GameSetupAction, GameSetupView};
use eframe::{App, CreationContext, Frame};
use egui::Context;
use hnefatafl::board::state::LargeBasicBoardState;
use std::process::exit;

enum View {
    GameSetup(GameSetupView),
    Game(GamePlayView<LargeBasicBoardState>)
}

pub(crate) struct MyApp {
    current_view: View,
}

impl MyApp {
    pub(crate) fn new(cc: &CreationContext) -> Self {
        Self {
            current_view: View::GameSetup(GameSetupView::default())
        }
    }
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