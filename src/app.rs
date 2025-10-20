use crate::game_play_view::{GamePlayAction, GamePlayView};
use crate::game_setup_view::{GameSetupAction, GameSetupView};
use eframe::{App, CreationContext, Frame};
use std::process::exit;
use egui::RichText;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use hnefatafl::aliases::LargeBasicBoardState;

enum View {
    GameSetup(GameSetupView),
    GamePlay(GamePlayView<LargeBasicBoardState>),
    About,
}

pub(crate) struct MyApp {
    current_view: View,
}

impl MyApp {
    pub(crate) fn new(cc: &CreationContext) -> Self {
        Self {
            current_view: View::GameSetup(GameSetupView::default()),
        }
    }

    fn about_view(&self, ctx: &egui::Context) -> bool {
        let mut back = false;
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.label(RichText::new("About this demo").heading());
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut cm_cache = CommonMarkCache::default();
            CommonMarkViewer::new().show(
                ui,
                &mut cm_cache,
                "This is a basic Hnefatafl app designed to demonstrate what you can build with \
                the `hnefatafl-rs` crate in Rust. You can play a few different variants against a \
                basic AI. It uses the `egui` GUI library and can be built as a native or web app.\n\n\
                * `hnefatafl-rs` crate on crates.io: <https://crates.io/crates/hnefatafl>\n\
                * `hnefatafl-rs` source code on GitHub: <https://github.com/bunburya/hnefatafl-rs>\n\
                * Source code for this demo app on GitHub: <https://github.com/bunburya/hnefatafl-egui>
                "
            );
            if ui.button("Back").clicked() {
                back = true;
            }
        });
        back
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        let new_view = match self.current_view {
            View::GameSetup(ref mut game_setup_view) => {
                // Game setup screen
                match game_setup_view.update(ctx) {
                    Some(GameSetupAction::StartGame(gs)) => {
                        Some(View::GamePlay(GamePlayView::new(gs)))
                    }
                    Some(GameSetupAction::ViewAbout) => Some(View::About),
                    Some(GameSetupAction::Quit) => exit(0),
                    None => None,
                }
            }
            View::GamePlay(ref mut game_play_view) => {
                // Game play screen
                match game_play_view.update(ctx) {
                    Some(GamePlayAction::QuitGame) => {
                        Some(View::GameSetup(GameSetupView::default()))
                    }
                    Some(GamePlayAction::QuitApp) => exit(0),
                    _ => None,
                }
            }
            View::About => {
                if self.about_view(ctx) {
                    Some(View::GameSetup(GameSetupView::default()))
                } else {
                    None
                }
            }
        };
        if let Some(view) = new_view {
            self.current_view = view;
        }
    }
}
