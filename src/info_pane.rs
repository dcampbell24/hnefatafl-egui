use hnefatafl::board::state::BoardState;
use hnefatafl::game::Game;

pub(crate) enum UserAction {
    Undo
}

pub(crate) struct InfoPane {
    
}

impl InfoPane {
    
    pub(crate) fn update<T: BoardState>(&mut self, game: &Game<T>, ctx: &egui::Context, ui: &mut egui::Ui) -> Option<UserAction> {
        let undo_button = ui.button("Undo");
        if undo_button.clicked() {
            return Some(UserAction::Undo);
        }
        let history = ui.label(game.play_history.iter().map(|pr| 
            pr.to_string()).collect::<Vec<String>>().join("\n")
        );
        None
    }
}