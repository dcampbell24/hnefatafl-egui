use std::cell::RefCell;
use crate::ai::AiError::{NoPlayAvailable, NotMyTurn};
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
use std::time::Duration;
use hnefatafl::pieces;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;


#[derive(Default)]
pub(crate) struct SearchStats {
    states: u32,
    paths: u32,
    tt_hits: u32,
    tt_replacements: u32,
    tt_inserts: u32,
    ab_prunes: u32,
    max_depth: u8
}

pub(crate) enum AiError {
    BoardError(BoardError),
    NoPlayAvailable,
    NotMyTurn
}

struct ZobristTable {
    /// Bitstrings representing piece placement
    piece_bits: Vec<[u64; 3]>,
    /// Bitstring to use used when it's the defender's move.
    def_to_move_bits: u64,
    board_len: u8
}

impl ZobristTable {

    fn new(board_len: u8, rng: &mut impl Rng) -> Self {
        let n_tiles = (board_len as usize).pow(2);
        let mut hashes: Vec<[u64; 3]> = Vec::with_capacity(n_tiles);
        for _ in 0..n_tiles {
            hashes.push([rng.next_u64(), rng.next_u64(), rng.next_u64()]);
        }
        Self {
            piece_bits: hashes,
            def_to_move_bits: rng.next_u64(),
            board_len
        }
    }

    fn piece_index(piece: Piece) -> usize {
        if piece.side == Attacker { 0 } else if piece.piece_type == Soldier { 1 } else { 2 }
    }

    fn hash<T: BoardState>(&self, board_state: T, side_to_play: pieces::Side) -> u64 {
        let mut h = 0u64;
        if side_to_play == Defender {
            h ^= self.def_to_move_bits;
        }
        for s in [Attacker, Defender] {
            for t in board_state.iter_occupied(s) {
                let bi = t.col as usize + (t.row as usize * self.board_len as usize);
                let p = board_state.get_piece(t).expect("There should be a piece here.");
                let pi = Self::piece_index(p);
                h ^= self.piece_bits[bi][pi];
            }
        }
        h
    }
}

#[derive(Debug)]
enum NodeType {
    LowerBound,
    UpperBound,
    Exact
}

#[derive(Debug)]
struct Node {
    depth: u8,
    score: i32,
    node_type: NodeType
}

pub(crate) struct TranspositionTable {
    table: HashMap<u64, Node>,
    size: usize
}

impl TranspositionTable {
    fn new(size: usize) -> Self {
        Self { table: HashMap::with_capacity(size), size }
    }
    
    fn insert(&mut self, hash: u64, data: Node) -> Option<Node> {
        self.table.insert(hash % self.size as u64, data)
    }

    fn get(&self, hash: u64) -> Option<&Node> {
        self.table.get(&(&hash % self.size as u64))
    }

    fn get_mut(&mut self, hash: u64) -> Option<&mut Node> {
        self.table.get_mut(&(&hash % self.size as u64))
    }
}

impl From<BoardError> for AiError {
    fn from(err: BoardError) -> AiError {
        AiError::BoardError(err)
    }
}

pub trait Ai {
    fn next_play<T: BoardState>(&mut self, game_state: &GameState<T>) -> Result<(Play, Vec<String>), AiError>;
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
    fn next_play<T: BoardState>(&mut self, game_state: &GameState<T>) -> Result<(Play, Vec<String>), AiError> {
        let mut plays: Vec<Play> = vec![];
        for t in game_state.board.iter_occupied(self.side) {
            for p in self.logic.iter_plays(t, game_state)? {
                plays.push(p)
            }
        }
        sleep(Duration::from_millis(500));
        Ok((plays[self.rng.next_u32() as usize % plays.len()], vec![]))
    }
}

pub struct BasicAi {
    side: Side,
    logic: GameLogic,
    zt: ZobristTable,
    tt: RefCell<TranspositionTable>,
    rng: ThreadRng,
    time_to_play: Duration
}

impl BasicAi {
    pub(crate) fn new(logic: GameLogic, side: Side, time_to_play: Duration) -> Self {
        let mut rng = thread_rng();
        Self {
            side,
            logic,
            zt: ZobristTable::new(logic.board_geo.side_len, &mut rng),
            // Smaller capacity on WASM
            #[cfg(target_arch = "wasm32")]
            tt: RefCell::from(TranspositionTable::new(2 << 16)),
            #[cfg(not(target_arch = "wasm32"))]
            tt: RefCell::from(TranspositionTable::new(2 << 28)),
            rng,
            time_to_play
        }
    }
    
    fn eval_board<T: BoardState>(&self, board: &T) -> i32 {
        let mut score = 0i32;
        let att_count = board.count_pieces(Attacker) as i32;
        let def_count = board.count_pieces(Defender) as i32;

        // More pieces a side has/fewer pieces the other side has = better for that side
        score += att_count * 10;
        score -= (def_count - 1) * 20;

        // More pieces on the board generally = better for attacker
        score += att_count + def_count;

        // King closer to edge = better for defender
        let side_len = self.logic.board_geo.side_len;
        let king_pos = board.get_king();
        let col_dist = min(king_pos.col, side_len - king_pos.col - 1);
        let row_dist = min(king_pos.row, side_len - king_pos.row - 1);
        score += (col_dist * 5) as i32;
        score += (row_dist * 5) as i32;

        // Fewer hostile pieces near king = better for defender
        score += (self.logic.board_geo.neighbors(king_pos).iter()
            .filter(|n| self.logic.tile_hostile(**n, Piece::new(King, Defender), board))
            .count() * 10) as i32;
        
        score
    }
    
    fn eval_state<T: BoardState>(&self, state: &GameState<T>, depth: u8) -> i32 {
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

        let mut score = self.eval_board(&state.board);
            
        // Penalise repetitions
        score -= (state.repetitions.get_repetitions(Attacker) * 10) as i32;
        score += (state.repetitions.get_repetitions(Defender) * 10) as i32;
        
        score
    }

    pub(crate) fn minimax<T: BoardState>(
        &self,
        play: Play,
        starting_state: GameState<T>,
        depth: u8,
        maximize: bool,
        mut alpha: i32,
        mut beta: i32,
        stats: &mut SearchStats
    ) -> i32 {
        stats.states += 1;
        let state = self.logic.do_play(play, starting_state).expect("Invalid play").0;
        let hash = self.zt.hash(state.board, state.side_to_play);
        if let Some(node) = self.tt.borrow().get(hash) {
            if node.depth >= depth {
                stats.tt_hits += 1;
                match node.node_type {
                    NodeType::Exact => return node.score,
                    NodeType::LowerBound if node.score >= beta => return beta,
                    NodeType::UpperBound if node.score <= alpha => return alpha,
                    _ => {}
                }
            }
        };
        if depth == 0 || state.status != Ongoing {
            stats.paths += 1;
            return self.eval_state(&state, depth);
        }
        
        let mut node_type = NodeType::Exact;
        let mut best_score = if maximize { i32::MIN } else { i32::MAX };
        if maximize {
            'outer: for t in state.board.iter_occupied(state.side_to_play) {
                for p in self.logic.iter_plays(t, &state).unwrap() {
                    let mm_score = self.minimax(p, state, depth-1, false, alpha, beta, stats);
                    if mm_score > best_score {
                        node_type = NodeType::Exact;
                        best_score = mm_score;
                    }
                    alpha = max(alpha, mm_score);
                    if alpha >= beta {
                        stats.ab_prunes += 1;
                        node_type = NodeType::LowerBound;
                        break 'outer
                    }
                }
            }
        } else {
            'outer: for t in state.board.iter_occupied(state.side_to_play) {
                for p in self.logic.iter_plays(t, &state).unwrap() {
                    let mm_score = self.minimax(p, state, depth-1, true, alpha, beta, stats);
                    if mm_score < best_score {
                        node_type = NodeType::Exact;
                        best_score = mm_score;
                    }
                    beta = min(beta, mm_score);
                    if alpha >= beta {
                        stats.ab_prunes += 1;
                        node_type = NodeType::UpperBound;
                        break 'outer
                    }
                }
            }
        }
        if let Some(_) = self.tt.borrow_mut().insert(hash, Node {
            depth,
            score: best_score,
            node_type,
        }) {
            stats.tt_replacements += 1;
        } else {
            stats.tt_inserts += 1;
        }
        best_score
    }

    /// Perform minimax search (with alpha beta pruning) up to the given depth.
    fn search_to_depth<T: BoardState>(
        &self,
        depth: u8,
        state: GameState<T>,
        maximize: bool,
        stats: &mut SearchStats,
        cutoff_time: Instant
    ) -> (Option<Play>, i32, bool) {

        let mut best_score = if maximize { i32::MIN } else { i32::MAX };
        let mut best_play: Option<Play> = None;
        
        for t in state.board.iter_occupied(self.side) {
            for p in self.logic.iter_plays(t, &state).expect("Cannot iterate plays.") {
                if Instant::now() > cutoff_time {
                    return (best_play, best_score, true);
                }
                // Not really sure why we need to negate maximize here but the algo definitely
                // performs better when we do...
                let s = self.minimax(p, state, depth, !maximize, i32::MIN, i32::MAX, stats);
                if maximize && (s > best_score) {
                    best_score = s;
                    best_play = Some(p);
                } else if (!maximize) && (s < best_score) {
                    best_score = s;
                    best_play = Some(p);
                }
            }
        }
        (best_play, best_score, false)
    }

    fn iddfs<T: BoardState>(
        &self,
        state: GameState<T>,
        maximize: bool,
        stats: &mut SearchStats
    ) -> (Option<Play>, i32) {
        let mut depth = 1;
        let mut best_play: Option<Play> = None;
        let mut best_score: i32 = if maximize { i32::MIN } else { i32::MAX };
        let start_time = Instant::now();
        loop {
            let (play, score, out_of_time) = self.search_to_depth(
                depth,
                state,
                maximize,
                stats,
                start_time + self.time_to_play
            );
            if let Some(p) = play {
                if !out_of_time {
                    println!("Best play after search depth {}: {} (score: {})", depth, p, score);
                    best_play = play;
                    best_score = score;

                }
            } 
            if out_of_time || play.is_none() {
                stats.max_depth = depth;
                return (best_play, best_score);
            }
            depth += 1
        }
    }
    
    fn lookup_tt<T: BoardState>(&self, state: &GameState<T>) -> Option<i32> {
        let hash = self.zt.hash(state.board, state.side_to_play);
        if let Some(node) = self.tt.borrow().get(hash) {
            Some(node.score)
        } else {
            None
        }
    }
}

impl Ai for BasicAi {
    fn next_play<T: BoardState>(&mut self, game_state: &GameState<T>) -> Result<(Play, Vec<String>), AiError> {
        if game_state.side_to_play != self.side {
            return Err(NotMyTurn)
        }
        let mut stats = SearchStats::default();
        let start_time = Instant::now();
        let (best_play, best_score) = self.iddfs(
            *game_state, 
            self.side == Attacker,
            &mut stats
        );
        
        let log_lines: Vec<String> = vec![
            format!("Searched {} paths ({} states) in {}s.",
                     stats.paths, stats.states, start_time.elapsed().as_secs_f32()),
            format!("Maximum depth searched: {}", stats.max_depth),
            format!("Pruned {} paths.", stats.ab_prunes),
            
            format!("TT hits: {}, insertions: {}, replacements: {}.", stats.tt_hits, stats.tt_inserts, stats.tt_replacements)
        ];
        
        if let Some(p) = best_play {
            println!("Best play: {p}, score: {best_score}");
            Ok((p, log_lines))
        } else {
            println!("No play found");
            Err(NoPlayAvailable)
        }
    }
}