use crate::ai::{Ai, BasicAi};
use crate::board::Board;
use eframe::emath::Align;
use egui::Layout;
use hnefatafl::board::state::BoardState;
use hnefatafl::game::state::GameState;
use hnefatafl::game::Game;
use hnefatafl::game::GameOutcome::{Draw, Win};
use hnefatafl::game::GameStatus::Over;
use hnefatafl::pieces;
use hnefatafl::play::{Play, PlayRecord};
use hnefatafl::rules::Ruleset;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::{
    thread,
    thread::JoinHandle
};
#[cfg(target_arch = "wasm32")]
use wasm_thread::{
    self as thread,
    JoinHandle
};

enum Message<T: BoardState> {
    Request(GameState<T>),
    Response(Play, GameState<T>, Vec<String>)
}

pub(crate) enum GamePlayAction {
    UndoPlay,
    QuitGame,
    QuitApp
}

pub(crate) struct GameSetup {
    pub(crate) ruleset: Ruleset,
    pub(crate) ruleset_name: String,
    pub(crate) starting_board: String,
    pub(crate) ai_side: pieces::Side,
    pub(crate) ai_time: Duration
}

pub(crate) struct GamePlayView<T: BoardState> {
    game: Game<T>,
    board_ui: Board,
    last_play: Option<PlayRecord>,
    ai_side: pieces::Side,
    ai_thread: JoinHandle<()>,
    ai_sender: std::sync::mpsc::Sender<Message<T>>,
    ai_receiver: std::sync::mpsc::Receiver<Message<T>>,
    log_lines: Vec<String>
}

impl<T: BoardState + Send + 'static> GamePlayView<T> {
    pub(crate) fn new(setup: GameSetup) -> Self {
        let game: Game<T> = Game::new(setup.ruleset, &setup.starting_board).unwrap();
        let board = Board::new(&game, setup.ai_side.other());
        let (g2ai_tx, g2ai_rx) = std::sync::mpsc::channel::<Message<T>>();
        let (ai2g_tx, ai2g_rx) = std::sync::mpsc::channel::<Message<T>>();
        let ai_thread = thread::spawn(move || {
            let mut ai = BasicAi::new(game.logic, setup.ai_side, setup.ai_time);
            loop {
                if let Ok(Message::Request(state)) = g2ai_rx.recv() {
                    if let Ok((play, lines)) = ai.next_play(&state) {
                        ai2g_tx.send(Message::Response(play, state, lines))
                            .expect("Failed to send response");
                        //ctx.request_repaint()
                    }
                } else {
                    break
                }
            }
        });
        if setup.ai_side == setup.ruleset.starting_side {
            g2ai_tx.send(Message::Request(game.state)).expect("Failed to send request");
        }
        let log_lines = vec![
            format!(
                "Game is {:?}. AI plays as {:?}, human plays as {:?}. {:?} to play first.",
                setup.ruleset_name,
                setup.ai_side,
                setup.ai_side.other(),
                setup.ruleset.starting_side
            )
        ];
        Self {
            game,
            board_ui: board,
            last_play: None,
            ai_side: setup.ai_side,
            ai_thread,
            ai_sender: g2ai_tx,
            ai_receiver: ai2g_rx,
            log_lines

        }
    }

    fn handle_play(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if let Ok(Message::Response(ai_play, state, mut lines)) = self.ai_receiver.try_recv() {
            self.log_lines.append(&mut lines);
            if state == self.game.state {
                self.game.do_play(ai_play).unwrap();
                self.log_lines.push(format!("{:?} played {}", self.ai_side, ai_play));
            }
        }
        if let Some(human_play) = self.board_ui.update(&self.game, ctx, ui) {
            self.game.do_play(human_play).unwrap();
            self.log_lines.push(format!("{:?} played {}", self.ai_side.other(), human_play));
            self.ai_sender.send(Message::Request(self.game.state))
                .expect("Failed to send request");
        }
        if let Over(outcome) = self.game.state.status {
            let over_msg = match outcome {
                Win(reason, side) =>
                    format!("{side:?} has won ({reason:?})."),
                Draw(reason) =>
                    format!("Draw ({reason:?}).")
            };
            if self.log_lines.last().is_some_and(|m| m != over_msg.as_str()) {
                self.log_lines.push(over_msg);
            }
        }
    }
    
    pub(crate) fn update(&mut self, ctx: &egui::Context) -> Option<GamePlayAction> {
        let mut action: Option<GamePlayAction> = None;
        egui::TopBottomPanel::bottom("log_pane").show(ctx, |ui| {
            ui.vertical(|ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(100.0).show(ui, |ui| {
                    ui.label(self.log_lines.join("\n").as_str());
                });
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    if ui.button("Quit game").clicked() {
                        action = Some(GamePlayAction::QuitGame)
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Quit app").clicked() {
                        action = Some(GamePlayAction::QuitApp)
                    }
                    let undo_button = ui.button("Undo move");
                    if undo_button.clicked() {
                        action = Some(GamePlayAction::UndoPlay);
                    }

                })
            })
        });
        egui::CentralPanel::default().show(&ctx, |ui| {
            self.handle_play(ctx, ui);
        });
        if let Some(GamePlayAction::UndoPlay) = action {
            self.game.undo_last_play();
            self.ai_sender.send(Message::Request(self.game.state))
                .expect("Failed to send request");

        }
        action
    }

}