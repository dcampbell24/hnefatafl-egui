use crate::game_play_view::GameSetup;
use egui::RichText;
use hnefatafl::pieces;
use hnefatafl::preset::{boards, rules};
use hnefatafl::rules::Ruleset;
use std::collections::HashMap;
use std::time::Duration;

pub(crate) enum GameSetupAction {
    StartGame(GameSetup),
    ViewAbout,
    Quit,
}

pub(crate) struct GameSetupView {
    variants: HashMap<String, (Ruleset, String)>,
    ai_sides: HashMap<String, pieces::Side>,
    ai_time: u8,
    selected_variant: String,
    selected_ai_side: String,
}

impl GameSetupView {
    pub(crate) fn new(
        variants: HashMap<String, (Ruleset, String)>,
        ai_sides: HashMap<String, pieces::Side>,
    ) -> Self {
        let mut variant_keys: Vec<String> = variants.keys().cloned().collect();
        variant_keys.sort();
        let selected_variant = variant_keys.first().expect("No variants provided.").clone();
        let mut side_keys: Vec<String> = ai_sides.keys().cloned().collect();
        side_keys.sort();
        let selected_ai_side = side_keys.first().expect("No sides provided.").clone();

        Self {
            variants,
            ai_sides,
            ai_time: 5,
            selected_variant,
            selected_ai_side,
        }
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context) -> Option<GameSetupAction> {
        let mut action: Option<GameSetupAction> = None;
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.label(RichText::new("Set up new game").heading());
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("game_setup_grid").show(ui, |ui| {
                ui.label("Variant:");
                egui::ComboBox::from_id_salt("variant")
                    .selected_text(&self.selected_variant)
                    .show_ui(ui, |combo_box| {
                        for k in self.variants.keys() {
                            combo_box.selectable_value(
                                &mut self.selected_variant,
                                k.clone(),
                                k.as_str(),
                            );
                        }
                    });
                ui.end_row();
                ui.label("AI side:");
                egui::ComboBox::from_id_salt("ai_side")
                    .selected_text(&self.selected_ai_side)
                    .show_ui(ui, |combo_box| {
                        for k in self.ai_sides.keys() {
                            combo_box.selectable_value(
                                &mut self.selected_ai_side,
                                k.clone(),
                                k.as_str(),
                            );
                        }
                    });
                ui.end_row();
                ui.label("AI time per move:");
                ui.add(egui::Slider::new(&mut self.ai_time, 1..=60));
                ui.end_row();
                if ui.button("Start game").clicked() {
                    let ruleset_name = self.selected_variant.clone();
                    let (ruleset, starting_board) = self.variants[&ruleset_name].clone();
                    action = Some(GameSetupAction::StartGame(GameSetup {
                        ruleset,
                        ruleset_name,
                        starting_board,
                        ai_side: self.ai_sides[&self.selected_ai_side],
                        ai_time: Duration::from_secs(self.ai_time as u64),
                    }));
                }
                if ui.button("About").clicked() {
                    action = Some(GameSetupAction::ViewAbout)
                }
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Quit").clicked() {
                    action = Some(GameSetupAction::Quit);
                }
            });
        });
        action
    }
}

impl Default for GameSetupView {
    fn default() -> Self {
        let mut variants: HashMap<String, (Ruleset, String)> = HashMap::default();
        variants.insert(
            "Copenhagen".to_string(),
            (rules::COPENHAGEN, boards::COPENHAGEN.to_string()),
        );
        variants.insert(
            "Brandubh".to_string(),
            (rules::BRANDUBH, boards::BRANDUBH.to_string()),
        );
        variants.insert(
            "Tablut".to_string(),
            (rules::TABLUT, boards::TABLUT.to_string()),
        );
        variants.insert(
            "Magpie".to_string(),
            (rules::MAGPIE, boards::MAGPIE.to_string()),
        );

        let mut sides: HashMap<String, pieces::Side> = HashMap::default();
        sides.insert("Attacker".to_string(), pieces::Side::Attacker);
        sides.insert("Defender".to_string(), pieces::Side::Defender);

        Self::new(variants, sides)
    }
}
