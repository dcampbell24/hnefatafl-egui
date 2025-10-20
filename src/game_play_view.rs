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
use hnefatafl::play::ValidPlay;
use hnefatafl::rules::Ruleset;
use std::cmp::min;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

enum Message<T: BoardState> {
    Request(GameState<T>),
    Response(ValidPlay, GameState<T>, Vec<String>)
}

pub(crate) enum GamePlayAction {
    UndoPlay,
    QuitGame,
    QuitApp,
}

pub(crate) struct GameSetup {
    pub(crate) ruleset: Ruleset,
    pub(crate) ruleset_name: String,
    pub(crate) starting_board: String,
    pub(crate) ai_side: pieces::Side,
    pub(crate) ai_time: Duration,
}

pub(crate) struct GamePlayView<T: BoardState> {
    game: Game<T>,
    board_ui: Board<T>,
    ai_side: pieces::Side,
    ai_sender: std::sync::mpsc::Sender<Message<T>>,
    ai_receiver: std::sync::mpsc::Receiver<Message<T>>,
    log_lines: Vec<String>,
}

impl<T: BoardState + Send + 'static> GamePlayView<T> where T::BitField: Send  {
    pub(crate) fn new(setup: GameSetup) -> Self {
        let game: Game<T> = Game::new(setup.ruleset, &setup.starting_board).unwrap();
        let board = Board::new(&game, setup.ai_side.other());
        let (g2ai_tx, g2ai_rx) = std::sync::mpsc::channel::<Message<T>>();
        let (ai2g_tx, ai2g_rx) = std::sync::mpsc::channel::<Message<T>>();
        thread::spawn(move || {
            let mut ai = BasicAi::new(game.logic, setup.ai_side, setup.ai_time);
            loop {
                if let Ok(Message::Request(state)) = g2ai_rx.recv() {
                    if let Ok((play, lines)) = ai.next_play(&state) {
                        // Don't panic if we can't send the response, it probably just means that
                        // the user has quit the game
                        let _ = ai2g_tx.send(Message::Response(play, state, lines));
                        //ctx.request_repaint()
                    }
                } else {
                    break;
                }
            }
        });
        if setup.ai_side == setup.ruleset.starting_side {
            let _ = g2ai_tx.send(Message::Request(game.state));
        }
        let log_lines = vec![format!(
            "Game is {:?}. AI plays as {:?}, human plays as {:?}. {:?} to play first.",
            setup.ruleset_name,
            setup.ai_side,
            setup.ai_side.other(),
            setup.ruleset.starting_side
        )];
        Self {
            game,
            board_ui: board,
            ai_side: setup.ai_side,
            ai_sender: g2ai_tx,
            ai_receiver: ai2g_rx,
            log_lines,
        }
    }

    fn handle_play(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, board_side_px: f32) {
        if let Ok(Message::Response(ai_play, state, mut lines)) = self.ai_receiver.try_recv() {
            self.log_lines.append(&mut lines);
            if state == self.game.state {
                let play_res = self.game.logic.do_valid_play(ai_play, state);
                self.game.state_history.push(play_res.new_state);
                self.game.state = play_res.new_state;
                self.game.play_history.push(play_res.record);
                self.log_lines.push(format!("{:?} played {}", self.ai_side, ai_play));
            }
        }
        if let Some(human_play) = self.board_ui.update(&self.game, ctx, ui, board_side_px) {
            self.game.do_play(human_play).unwrap();
            self.log_lines
                .push(format!("{:?} played {}", self.ai_side.other(), human_play));
            self.ai_sender
                .send(Message::Request(self.game.state))
                .expect("Failed to send request");
        }
        if let Over(outcome) = self.game.state.status {
            let over_msg = match outcome {
                Win(reason, side) => format!("{side:?} has won ({reason:?})."),
                Draw(reason) => format!("Draw ({reason:?})."),
            };
            if self
                .log_lines
                .last()
                .is_some_and(|m| m != over_msg.as_str())
            {
                self.log_lines.push(over_msg);
            }
        }
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context) -> Option<GamePlayAction> {
        let mut action: Option<GamePlayAction> = None;
        let total_space = ctx.screen_rect();
        // Bottom panel (with logs and buttons) gets 25% of screen height
        let bottom_panel_height = total_space.max.y * 0.25;
        // Central panel (with board) gets 75% of screen height or 100% of screen width, whichever
        // is smaller (as it has to be a square)
        let central_panel_side = min(
            (total_space.max.y - bottom_panel_height) as u32,
            total_space.max.x as u32,
        ) as f32;

        egui::TopBottomPanel::bottom("log_pane")
            .exact_height(bottom_panel_height)
            .show(ctx, |ui| {
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.horizontal(|ui| {
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
                    });
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, true])
                            //.max_height(bottom_panel_height)
                            .show(ui, |ui| {
                                ui.label(self.log_lines.join("\n").as_str());
                                ui.scroll_to_cursor(Some(Align::BOTTOM))
                            });
                    })
                })
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.handle_play(ctx, ui, central_panel_side);
        });
        if let Some(GamePlayAction::UndoPlay) = action {
            self.game.undo_last_play();
            self.ai_sender
                .send(Message::Request(self.game.state))
                .expect("Failed to send request");
        }
        action
    }
}
