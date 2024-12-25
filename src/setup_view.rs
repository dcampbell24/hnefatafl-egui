use crate::game_view::GameSetup;
use hnefatafl::pieces;
use hnefatafl::rules::Ruleset;
use std::collections::HashMap;
use std::time::Duration;

pub(crate) enum GameSetupAction {
    StartGame(GameSetup)
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
        let variant_keys: Vec<String> = variants.keys().cloned().collect();
        let selected_variant = variant_keys.first().expect("No variants provided.").clone();
        let side_keys: Vec<String> = ai_sides.keys().cloned().collect();
        let selected_ai_side = side_keys.first().expect("No sides provided.").clone();

        Self { variants, ai_sides, ai_time: 1, selected_variant, selected_ai_side }
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) -> Option<GameSetupAction> {
        let mut action: Option<GameSetupAction> = None;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("game_setup_grid").show(ui, |ui| {
                ui.label("Variant:");
                egui::ComboBox::from_id_salt("variant")
                    .selected_text(&self.selected_variant)
                    .show_ui(ui, |combo_box| {
                        for (k, v) in &self.variants {
                            combo_box.selectable_value(&mut self.selected_variant, k.clone(), k.as_str());
                        }
                    });
                ui.end_row();
                ui.label("AI side:");
                egui::ComboBox::from_id_salt("ai_side")
                    .selected_text(&self.selected_ai_side)
                    .show_ui(ui, |combo_box| {
                        for (k, _) in &self.ai_sides {
                            combo_box.selectable_value(&mut self.selected_ai_side, k.clone(), k.as_str());
                        }
                    });
                ui.end_row();
                ui.label("AI time per move:");
                ui.add(egui::Slider::new(&mut self.ai_time, 1..=60));
                ui.end_row();
                if ui.button("Start game").clicked() {
                    let (ruleset, starting_board) = self.variants[&self.selected_variant].clone();
                    action = Some(GameSetupAction::StartGame(GameSetup {
                            ruleset,
                            starting_board,
                            ai_side: self.ai_sides[&self.selected_ai_side],
                            ai_time: Duration::from_secs(self.ai_time as u64),
                    }))
                }
            });
        });
        action
    }

}