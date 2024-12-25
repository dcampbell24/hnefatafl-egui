mod ai;
mod board;
mod info_pane;
mod game_view;
mod setup_view;

use std::collections::HashMap;
use crate::game_view::GameView;
use crate::setup_view::{GameSetupAction, GameSetupView};
use eframe::{App, Frame};
use egui::Context;
use hnefatafl::board::state::{BoardState, LargeBasicBoardState};
use hnefatafl::pieces;
use hnefatafl::preset::{boards, rules};
use hnefatafl::rules::Ruleset;
use crate::View::Game;

enum View {
    GameSetup(GameSetupView),
    Game(GameView<LargeBasicBoardState>)
}

struct MyApp {
    current_view: View,
}

impl App for MyApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let new_view = match self.current_view {
            View::GameSetup(ref mut game_setup_view) => {
                if let Some(GameSetupAction::StartGame(gs)) = game_setup_view.update(ctx, frame) {
                    println!("Got action");
                    Some(Game(GameView::new(ctx, gs)))
                } else {
                    None
                }
            },
            View::Game(ref mut game_view) => {
                game_view.update(ctx, frame);
                None
            }
        };
        if let Some(view) = new_view {
            self.current_view = view;
        }
    }
}

fn main() {
    let mut variants: HashMap<String, (Ruleset, String)> = HashMap::default();
    variants.insert("Copenhagen".to_string(), (rules::COPENHAGEN, boards::COPENHAGEN.to_string()));
    variants.insert("Brandubh".to_string(), (rules::BRANDUBH, boards::BRANDUBH.to_string()));
    variants.insert("Tablut".to_string(), (rules::TABLUT, boards::TABLUT.to_string()));
    
    let mut sides: HashMap<String, pieces::Side> = HashMap::default();
    sides.insert("Attacker".to_string(), pieces::Side::Attacker);
    sides.insert("Defender".to_string(), pieces::Side::Defender);
    
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Tafl egui demo",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp {
                // current_view: View::Game(GameView::<LargeBasicBoardState>::new(cc, GameSetup {
                //     ruleset: rules::TABLUT,
                //     starting_board: boards::TABLUT,
                //     ai_side: Defender,
                //     ai_time: Duration::from_secs(5)
                //     }))
                current_view: View::GameSetup(GameSetupView::new(
                    variants,
                    sides,
                )),
            }))})
    ).unwrap();
}
