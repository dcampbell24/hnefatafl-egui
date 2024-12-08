use crate::ai::AiError::NoPlayAvailable;
use hnefatafl::board::state::BoardState;
use hnefatafl::error::BoardError;
use hnefatafl::game::logic::GameLogic;
use hnefatafl::game::state::GameState;
use hnefatafl::game::GameOutcome::{Draw, Win};
use hnefatafl::game::GameStatus::{Ongoing, Over};
use hnefatafl::pieces::PieceType::{King, Soldier};
use hnefatafl::pieces::Side::{Attacker, Defender};
use hnefatafl::pieces::{Piece, Side};
use hnefatafl::play::Play;
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng, RngCore};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct SearchStats {
    states: u32,
    paths: u32
}

pub(crate) enum AiError {
    BoardError(BoardError),
    NoPlayAvailable
}

struct ZobristTable {
    hashes: Vec<[u64; 3]>,
    def_to_move: u64,
    board_len: usize
}

impl ZobristTable {

    fn new(board_len: usize, rng: &mut impl Rng) -> Self {
        let n_tiles = board_len.pow(2);
        let mut hashes: Vec<[u64; 3]> = Vec::with_capacity(n_tiles);
        for _ in 0..n_tiles {
            hashes.push([rng.next_u64(), rng.next_u64(), rng.next_u64()]);
        }
        Self { hashes, def_to_move: rng.next_u64(), board_len }
    }

    fn piece_index(piece: Piece) -> usize {
        if piece.side == Attacker { 0 } else if piece.piece_type == Soldier { 1 } else { 2 }
    }

    fn hash<T: BoardState>(&self, board_state: T, side_to_move: Side) -> u64 {
        let mut h = 0u64;
        if side_to_move == Defender {
            h ^= self.def_to_move;
        }
        for t in board_state.iter_occupied(Attacker) {
            let bi = t.col as usize + (t.row as usize * self.board_len);
            let p = board_state.get_piece(t).expect("There should be a piece here.");
            let pi = Self::piece_index(p);
            h ^= self.hashes[bi][pi];
        }
        h
    }
}

struct TranspositionTable {
    table: HashMap<u64, (i32, u8)>, // score, depth
    size: usize
}

impl TranspositionTable {
    fn new(size: usize) -> Self {
        Self { table: HashMap::with_capacity(size), size }
    }
    
    fn insert(&mut self, hash: u64, data: (i32, u8)) -> Option<(i32, u8)> {
        self.table.insert(hash % self.size as u64, data)
    }
}

impl From<BoardError> for AiError {
    fn from(err: BoardError) -> AiError {
        AiError::BoardError(err)
    }
}

pub trait Ai {
    fn next_play<T: BoardState>(&mut self, game_state: &GameState<T>) -> Result<Play, AiError>;
}

pub struct RandomAi {
    side: Side,
    logic: GameLogic,
    rng: ThreadRng
}

impl RandomAi {
    pub(crate) fn new(logic: GameLogic, side: Side) -> Self {
        Self { side, logic, rng: thread_rng() }
    }
}

impl Ai for RandomAi {
    fn next_play<T: BoardState>(&mut self, game_state: &GameState<T>) -> Result<Play, AiError> {
        let mut plays: Vec<Play> = vec![];
        for t in game_state.board.iter_occupied(self.side) {
            for p in self.logic.iter_plays(t, game_state)? {
                plays.push(p)
            }
        }
        sleep(Duration::from_millis(500));
        Ok(plays[self.rng.next_u32() as usize % plays.len()])
    }
}

pub struct BasicAi {
    side: Side,
    logic: GameLogic,
    rng: ThreadRng
}

impl BasicAi {
    pub(crate) fn new(logic: GameLogic, side: Side) -> Self {
        Self { side, logic, rng: thread_rng() }
    }
    
    fn eval<T: BoardState>(&self, state: &GameState<T>, depth: u8) -> i32 {

        if let Over(Win(_, winner)) = state.status {
            // prox_penalty is larger the further down the tree we had to search to get the win.
            // Used to promote quick wins/slow losses
            let prox_penalty = (u8::MAX as i32) - (depth as i32);
            return if winner == Attacker {
                i32::MAX - prox_penalty
            } else {
                i32::MIN + prox_penalty
            }
        } else if let Over(Draw(_)) = state.status {
            return 0
        }

        let mut score = 0i32;
        let att_count = state.board.count_pieces(Attacker) as i32;
        let def_count = state.board.count_pieces(Defender) as i32;

        // More pieces a side has/fewer pieces the other side has = better for that side
        score += att_count * 10;
        score -= (def_count - 1) * 20;

        // More pieces on the board generally = better for attacker
        score += att_count + def_count;

        // King closer to edge = better for defender
        let side_len = self.logic.board_geo.side_len;
        let king_pos = state.board.get_king();
        let col_dist = min(king_pos.col, side_len - king_pos.col - 1);
        let row_dist = min(king_pos.row, side_len - king_pos.row - 1);
        score += (col_dist * 5) as i32;
        score += (row_dist * 5) as i32;

        // Fewer hostile pieces near king = better for defender
        score += (self.logic.board_geo.neighbors(king_pos).iter()
            .filter(|n| self.logic.tile_hostile(**n, Piece::new(King, Defender), &state.board))
            .count() * 10) as i32;

        // Penalise repetitions
        score -= (state.repetitions.get_repetitions(Attacker) * 10) as i32;
        score += (state.repetitions.get_repetitions(Defender) * 10) as i32;
        
        score
    }

    pub(crate) fn minimax<T: BoardState>(
        &self,
        state: &GameState<T>,
        depth: u8,
        maximize: bool,
        mut alpha: i32,
        mut beta: i32,
        stats: &mut SearchStats
    ) -> i32 {
        stats.states += 1;
        if depth == 0 || state.status != Ongoing {
            stats.paths += 1;
            return self.eval(state, depth);
        }
        if maximize {
            let mut val = i32::MIN;
            for t in state.board.iter_occupied(state.side_to_play) {
                for p in self.logic.iter_plays(t, state).unwrap() {
                    let new_state = self.logic.do_play(p, *state).unwrap().0;
                    let mm_score = self.minimax(&new_state, depth-1, false, alpha, beta, stats);
                    val = max(val, mm_score);
                    alpha = max(alpha, val);
                    if beta <= alpha {
                        break
                    }
                }
            }
            val
        } else {
            let mut val = i32::MAX;
            for t in state.board.iter_occupied(state.side_to_play) {
                for p in self.logic.iter_plays(t, state).unwrap() {
                    let new_state = self.logic.do_valid_play(p, *state).0;
                    let mm_score = self.minimax(&new_state, depth-1, true, alpha, beta, stats);
                    val = min(val, mm_score);
                    beta = min(beta, val);
                    if beta <= alpha {
                        break
                    }
                }
            }
            val
        }
    }
}

impl Ai for BasicAi {
    fn next_play<T: BoardState>(&mut self, game_state: &GameState<T>) -> Result<Play, AiError> {
        let (mut benchmark, cmp_fn, maximize): (i32, fn(i32, i32) -> bool, bool) =
            if self.side == Defender {
                (i32::MAX, |x, y| x < y, true)
            } else {
                (i32::MIN, |x, y| x > y, false)
            };
        let mut best: Option<Play> = None;
        let mut stats = SearchStats::default();
        let start = Instant::now();
        for t in game_state.board.iter_occupied(self.side) {
            for p in self.logic.iter_plays(t, game_state)? {
                let state = self.logic.do_valid_play(p, *game_state).0;
                // NB: Because we have already executed the play being assessed, the first run of
                // minimax will be the other player's turn.
                let score = self.minimax(&state, 2, maximize, i32::MIN, i32::MAX, &mut stats);
                if best == None || cmp_fn(score, benchmark) {
                    benchmark = score;
                    best = Some(p);
                }
            }
        }
        println!("Searched {} paths ({} states) in {}s.",
                 stats.paths, stats.states, start.elapsed().as_secs_f32());

        if let Some(p) = best {
            println!("Best play: {p}, score: {benchmark}");
            Ok(p)
        } else {
            Err(NoPlayAvailable)
        }
    }
}