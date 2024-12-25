use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use egui::panel;
use hnefatafl::board::state::BoardState;
use hnefatafl::game::Game;
use hnefatafl::game::state::GameState;
use hnefatafl::pieces;
use hnefatafl::pieces::Side::Defender;
use hnefatafl::play::{Play, PlayRecord};
use hnefatafl::rules::Ruleset;
use crate::ai::{Ai, BasicAi};
use crate::board::Board;
use crate::info_pane::{InfoPane, UserAction};

enum Message<T: BoardState> {
    Request(GameState<T>, egui::Context),
    Response(Play, GameState<T>)
}

pub(crate) struct GameSetup {
    pub(crate) ruleset: Ruleset,
    pub(crate) starting_board: String,
    pub(crate) ai_side: pieces::Side,
    pub(crate) ai_time: Duration
}

pub(crate) struct GameView<T: BoardState> {
    game: Game<T>,
    board_ui: Board,
    info_pane_ui: InfoPane,
    last_play: Option<PlayRecord>,
    ai_side: pieces::Side,
    ai_thread: JoinHandle<()>,
    ai_sender: std::sync::mpsc::Sender<Message<T>>,
    ai_receiver: std::sync::mpsc::Receiver<Message<T>>,
}

impl<T: BoardState + Send + 'static> GameView<T> {
    pub(crate) fn new(egui_ctx: &egui::Context, setup: GameSetup) -> Self {
        let game: Game<T> = Game::new(setup.ruleset, &setup.starting_board).unwrap();
        let board = Board::new(&game, setup.ai_side.other());
        let info_pane = InfoPane {};
        let (g2ai_tx, g2ai_rx) = std::sync::mpsc::channel::<Message<T>>();
        let (ai2g_tx, ai2g_rx) = std::sync::mpsc::channel::<Message<T>>();
        let ai_thread = thread::spawn(move || {
            let mut ai = BasicAi::new(game.logic, setup.ai_side, setup.ai_time);
            loop {
                if let Ok(Message::Request(state, ctx)) = g2ai_rx.recv() {
                    if let Ok(play) = ai.next_play(&state) {
                        ai2g_tx.send(Message::Response(play, state))
                            .expect("Failed to send response");
                        ctx.request_repaint()
                    }
                } else {
                    break
                }
            }
        });
        if setup.ai_side == setup.ruleset.starting_side {
            g2ai_tx.send(Message::Request(game.state, egui_ctx.clone())).expect("Failed to send request");
        }
        Self {
            game,
            board_ui: board,
            info_pane_ui: info_pane,
            last_play: None,
            ai_side: Defender,
            ai_thread,
            ai_sender: g2ai_tx,
            ai_receiver: ai2g_rx

        }
    }

    fn handle_play(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if let Ok(Message::Response(ai_play, state)) = self.ai_receiver.try_recv() {
            if state == self.game.state {
                self.game.do_play(ai_play).unwrap();
            }
        }
        if let Some(human_play) = self.board_ui.update(&self.game, ctx, ui) {
            self.game.do_play(human_play).unwrap();
            self.ai_sender.send(Message::Request(self.game.state, ctx.clone()))
                .expect("Failed to send request");
        }
    }

    fn handle_info_pane(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        match self.info_pane_ui.update(&self.game, ctx, ui) {
            Some(UserAction::Undo) => self.game.undo_last_play(),
            _ => {}
        }
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::new(panel::Side::Right, "info_pane").show(ctx, |ui| {
            self.handle_info_pane(ctx, ui);
        });
        egui::CentralPanel::default().show(&ctx, |ui| {
            self.handle_play(ctx, ui);
        });
    }

}