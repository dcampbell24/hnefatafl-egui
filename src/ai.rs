use crate::ai::AiError::{NoPlayAvailable, NotMyTurn};
use hnefatafl::board::state::BoardState;
use hnefatafl::collections::PieceMap;
use hnefatafl::game::logic::GameLogic;
use hnefatafl::game::state::GameState;
use hnefatafl::game::GameOutcome::{Draw, Win};
use hnefatafl::game::GameStatus::{Ongoing, Over};
use hnefatafl::pieces;
use hnefatafl::pieces::PieceType::{King, Soldier};
use hnefatafl::pieces::Side::{Attacker, Defender};
use hnefatafl::pieces::{Piece, Side, KING};
use hnefatafl::play::ValidPlay;
use hnefatafl::tiles::Coords;
use rand::{thread_rng, Rng};
use std::cmp::min;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[derive(Default)]
pub(crate) struct SearchStats {
    states: u32,
    paths: u32,
    tt_hits: u32,
    tt_replacements: u32,
    tt_inserts: u32,
    ab_prunes: u32,
    max_depth: u8,
}

pub enum AiError {
    NoPlayAvailable,
    NotMyTurn,
}

struct ZobristTable {
    /// Bitstrings representing piece placement
    piece_bits: Vec<[u64; 3]>,
    /// Bitstring to use used when it's the defender's move.
    def_to_move_bits: u64,
    board_len: u8,
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
            board_len,
        }
    }

    fn piece_index(piece: Piece) -> usize {
        if piece.side == Attacker {
            0
        } else if piece.piece_type == Soldier {
            1
        } else {
            2
        }
    }

    fn hash<T: BoardState>(&self, board_state: T, side_to_play: pieces::Side) -> u64 {
        let mut h = 0u64;
        if side_to_play == Defender {
            h ^= self.def_to_move_bits;
        }
        for s in [Attacker, Defender] {
            for t in board_state.occupied_by_side(s) {
                let bi = t.col as usize + (t.row as usize * self.board_len as usize);
                let p = board_state
                    .get_piece(t)
                    .expect("There should be a piece here.");
                let pi = Self::piece_index(p);
                h ^= self.piece_bits[bi][pi];
            }
        }
        h
    }
}

#[derive(Debug, Clone, Copy)]
enum NodeType {
    LowerBound,
    UpperBound,
    Exact,
}

#[derive(Clone, Copy, Debug)]
struct TTEntry {
    hash: u64,
    depth: u8,
    score: i32,
    node_type: NodeType,
    best_play: Option<ValidPlay>,
    age: u8,
}

impl TTEntry {
    fn new(
        hash: u64,
        depth: u8,
        score: i32,
        node_type: NodeType,
        best_play: Option<ValidPlay>,
        age: u8,
    ) -> Self {
        Self {
            hash,
            depth,
            score,
            node_type,
            best_play,
            age,
        }
    }
}

pub(crate) struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    size: usize,
    current_age: u8,
}

impl TranspositionTable {
    fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TTEntry>>();
        let n_entries = (size_mb * 1024 * 1024) / entry_size;
        Self {
            entries: vec![None; n_entries],
            size: n_entries,
            current_age: 0,
        }
    }

    fn new_search(&mut self) {
        self.current_age = self.current_age.wrapping_add(1);
    }

    fn get_index(&self, hash: u64) -> usize {
        (hash as usize) % self.size
    }

    fn insert(
        &mut self,
        hash: u64,
        depth: u8,
        score: i32,
        node_type: NodeType,
        best_play: Option<ValidPlay>,
        stats: &mut SearchStats,
    ) {
        let index = self.get_index(hash);
        let entry = TTEntry::new(hash, depth, score, node_type, best_play, self.current_age);

        if let Some(existing) = &self.entries[index] {
            // Replace if:
            // 1. New entry is from current search (newer age) and old entry is from previous search
            // 2. New entry has greater or equal depth
            // 3. Hash collision occurred (different position)
            if (entry.age != existing.age && entry.age == self.current_age)
                || (entry.depth >= existing.depth)
                || (existing.hash != hash)
            {
                self.entries[index] = Some(entry);
                stats.tt_replacements += 1;
            }
        } else {
            self.entries[index] = Some(entry);
            stats.tt_inserts += 1;
        }
    }

    fn probe(&self, hash: u64) -> Option<TTEntry> {
        let index = self.get_index(hash);
        self.entries[index].and_then(|e| {
            // Verify correct hash (in case of collision)
            if e.hash == hash {
                Some(e)
            } else {
                None
            }
        })
    }
}

pub trait Ai {
    type BoardState: BoardState;
    fn next_play(
        &mut self,
        game_state: &GameState<Self::BoardState>,
    ) -> Result<(ValidPlay, Vec<String>), AiError>;
}

pub struct BasicAi<T: BoardState> {
    side: Side,
    logic: GameLogic<T>,
    zt: ZobristTable,
    tt: TranspositionTable,
    time_to_play: Duration,
}

impl<T: BoardState> BasicAi<T> {
    pub fn new(logic: GameLogic<T>, side: Side, time_to_play: Duration) -> Self {
        let mut rng = thread_rng();
        Self {
            side,
            logic,
            zt: ZobristTable::new(logic.board_geo.side_len, &mut rng),
            // Smaller capacity on WASM
            #[cfg(target_arch = "wasm32")]
            tt: TranspositionTable::new(128),
            #[cfg(not(target_arch = "wasm32"))]
            tt: TranspositionTable::new(512),
            time_to_play,
        }
    }

    /// Evaluate board state and return a score. Higher = better for attacker, lower = better for
    /// defender.
    fn eval_board(&self, board: &T) -> i32 {
        // unwrap should be safe here because we have already checked for win conditions (including
        // capture of king) in `eval_state`
        let king_tile = board
            .get_king()
            .expect("There should be a king on the board.");
        let king_coords = Coords::from(king_tile);

        let mut score = 0i32;
        let att_count = board.count_pieces_of_side(Attacker) as i32;
        let def_count = board.count_pieces_of_side(Defender) as i32;

        // More pieces a side has/fewer pieces the other side has = better for that side
        score += att_count * 10;
        score -= (def_count - 1) * 20;

        // More pieces on the board generally = better for attacker
        score += att_count + def_count;

        // King closer to edge = better for defender
        let side_len = self.logic.board_geo.side_len;
        let col_dist = min(king_tile.col, side_len - king_tile.col - 1);
        let row_dist = min(king_tile.row, side_len - king_tile.row - 1);
        score += (col_dist * 5) as i32;
        score += (row_dist * 5) as i32;

        // Fewer hostile pieces near king = better for defender
        score += (self
            .logic
            .board_geo
            .neighbors(king_tile)
            .iter()
            .filter(|n| {
                self.logic
                    .tile_hostile(**n, Piece::new(King, Defender), board)
            })
            .count()
            * 10) as i32;

        // Attacker pieces closer to king = better for attacker
        let mut total_dist = 0u32;
        let mut attacker_count = 0u32;
        for tile in board.occupied_by_side(Attacker) {
            total_dist += Coords::from(tile)
                .row_col_offset_from(king_coords)
                .manhattan_dist() as u32;
            attacker_count += 1;
        }
        score -= ((total_dist / attacker_count) as i32) * 10;

        score
    }

    /// Evaluate game state (board state + repetitions) and return a score. Higher = better for
    /// attacker, lower = better for defender.
    fn eval_state(&self, state: &GameState<T>, depth: u8) -> i32 {
        if let Over(Win(_, winner)) = state.status {
            // prox_penalty is larger the further down the tree we had to search to get the win.
            // Used to promote quick wins/slow losses
            let prox_penalty = (u8::MAX as i32) - (depth as i32);
            return if winner == Attacker {
                i32::MAX - prox_penalty
            } else {
                i32::MIN + prox_penalty
            };
        } else if let Over(Draw(_)) = state.status {
            return 0;
        }

        let mut score = self.eval_board(&state.board);

        // Penalise repetitions
        score -= (state.repetitions.get_repetitions(Attacker) * 10) as i32;
        score += (state.repetitions.get_repetitions(Defender) * 10) as i32;

        score
    }

    /// Quickly evaluate a play. Used in play ordering.
    fn eval_play(&self, vp: ValidPlay, state: &GameState<T>) -> i32 {
        let mut score = 0i32;
        let to = vp.play.to();
        let board = &state.board;
        let moving_piece = board.get_piece(vp.play.from).expect("No piece to move.");

        // Prioritise capture plays
        score += (self
            .logic
            .get_captures(vp, moving_piece, state)
            .occupied()
            .count() as i32)
            * 1000;

        // King-specific plays
        if moving_piece == KING {
            // Bonus for king moves towards edges (escape routes)
            let to_edge_dist = min(
                min(to.row, self.logic.board_geo.side_len - 1 - to.row),
                min(to.col, self.logic.board_geo.side_len - 1 - to.col),
            );
            score += (4 - to_edge_dist as i32) * 300;

            // Penalty for moving king next to attackers
            let hostile_neighbors = self
                .logic
                .board_geo
                .neighbors(to)
                .iter()
                .filter(|pos| board.get_piece(**pos).is_some_and(|p| p.side == Attacker))
                .count();
            score -= (hostile_neighbors as i32) * 400;
        }

        // 3. Mobility scoring
        let mobility = self
            .logic
            .board_geo
            .neighbors(to)
            .iter()
            .filter(|pos| board.get_piece(**pos).is_none())
            .count();
        score += (mobility as i32) * 50;

        score
    }

    fn order_plays(
        &self,
        valid_plays: Vec<ValidPlay>,
        state: &GameState<T>,
        tt_play: Option<ValidPlay>,
    ) -> Vec<ValidPlay> {
        let mut scored_plays: Vec<(ValidPlay, i32)> = valid_plays
            .into_iter()
            .map(|p| (p, self.eval_play(p, state)))
            .collect();

        // If we have a TT move, give it maximum priority
        if let Some(tp) = tt_play {
            if let Some(pos) = scored_plays.iter().position(|(p, _)| p == &tp) {
                scored_plays[pos].1 = i32::MAX;
            }
        }

        scored_plays.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        scored_plays.into_iter().map(|ps| ps.0).collect()
    }

    /// The minimax algorithm. Returns (best score, best play) tuple.
    pub(crate) fn minimax(
        &mut self,
        vp: ValidPlay,
        starting_state: GameState<T>,
        depth: u8,
        maximize: bool,
        mut alpha: i32,
        mut beta: i32,
        stats: &mut SearchStats,
    ) -> (i32, Option<ValidPlay>) {
        stats.states += 1;
        let state = self.logic.do_valid_play(vp, starting_state).new_state;
        let hash = self.zt.hash(state.board, state.side_to_play);

        if let Some(tt_entry) = self.tt.probe(hash) {
            // Found entry in transposition table
            if tt_entry.depth > depth {
                stats.tt_hits += 1;
                match tt_entry.node_type {
                    NodeType::Exact => return (tt_entry.score, tt_entry.best_play),
                    NodeType::LowerBound if tt_entry.score >= beta => {
                        return (beta, tt_entry.best_play)
                    }
                    NodeType::UpperBound if tt_entry.score <= alpha => {
                        return (alpha, tt_entry.best_play)
                    }
                    _ => {}
                }
            }
        }

        if depth == 0 || state.status != Ongoing {
            // Leaf node
            stats.paths += 1;
            return (self.eval_state(&state, depth), None);
        }

        let mut node_type = NodeType::Exact;
        let mut best_score = if maximize { i32::MIN } else { i32::MAX };
        let mut best_play: Option<ValidPlay> = None;

        // Collect and sort moves
        let mut plays = Vec::new();
        for t in state.board.occupied_by_side(state.side_to_play) {
            for p in self
                .logic
                .iter_plays(t, &state)
                .expect("Could not iterate plays")
            {
                plays.push(p);
            }
        }

        let tt_play = self.tt.probe(hash).and_then(|entry| entry.best_play);
        let plays = self.order_plays(plays, &state, tt_play);

        if maximize {
            for p in plays {
                let (score, _) = self.minimax(p, state, depth - 1, false, alpha, beta, stats);
                if score > best_score {
                    node_type = NodeType::Exact;
                    best_score = score;
                    best_play = Some(p);
                }
                alpha = alpha.max(score);
                if alpha >= beta {
                    stats.ab_prunes += 1;
                    node_type = NodeType::LowerBound;
                    break;
                }
            }
        } else {
            for p in plays {
                let (score, _) = self.minimax(p, state, depth - 1, true, alpha, beta, stats);
                if score < best_score {
                    node_type = NodeType::Exact;
                    best_score = score;
                    best_play = Some(p);
                }
                beta = beta.min(score);
                if alpha >= beta {
                    stats.ab_prunes += 1;
                    node_type = NodeType::UpperBound;
                    break;
                }
            }
        }

        // Store in transposition table
        self.tt
            .insert(hash, depth, best_score, node_type, best_play, stats);

        (best_score, best_play)
    }

    /// Perform minimax search (with alpha beta pruning) up to the given depth.
    fn search_to_depth(
        &mut self,
        depth: u8,
        state: GameState<T>,
        maximize: bool,
        stats: &mut SearchStats,
        cutoff_time: Instant,
    ) -> (Option<ValidPlay>, i32, bool) {
        let mut plays: Vec<(ValidPlay, GameState<T>)> = Vec::new();
        for t in state.board.occupied_by_side(state.side_to_play) {
            for p in self
                .logic
                .iter_plays(t, &state)
                .expect("Could not iterate plays")
            {
                let next_state = self.logic.do_valid_play(p, state).new_state;
                plays.push((p, next_state));
            }
        }

        let mut best_score = if maximize { i32::MIN } else { i32::MAX };
        let mut best_play: Option<ValidPlay> = None;

        for (vp, _) in plays {
            if Instant::now() > cutoff_time {
                return (best_play, best_score, true);
            }
            // Not really sure why we need to negate maximize here but the algo definitely
            // performs better when we do...
            let (score, _) = self.minimax(vp, state, depth, !maximize, i32::MIN, i32::MAX, stats);
            if maximize && (score > best_score) {
                best_score = score;
                best_play = Some(vp);
            } else if (!maximize) && (score < best_score) {
                best_score = score;
                best_play = Some(vp);
            }
        }

        (best_play, best_score, false)
    }

    fn iddfs(
        &mut self,
        state: GameState<T>,
        maximize: bool,
        stats: &mut SearchStats,
    ) -> (Option<ValidPlay>, i32) {
        self.tt.new_search();
        let mut depth = 1;
        let mut best_play: Option<ValidPlay> = None;
        let mut best_score: i32 = if maximize { i32::MIN } else { i32::MAX };
        let start_time = Instant::now();
        loop {
            let (play, score, out_of_time) = self.search_to_depth(
                depth,
                state,
                maximize,
                stats,
                start_time + self.time_to_play,
            );
            if let Some(p) = play {
                if !out_of_time {
                    println!(
                        "Best play after search depth {}: {} (score: {})",
                        depth, p, score
                    );
                    best_play = play;
                    best_score = score;
                }
            }
            if out_of_time || play.is_none() {
                if out_of_time {
                    stats.max_depth = depth - 1;
                } else {
                    stats.max_depth = depth;
                }
                return (best_play, best_score);
            }
            depth += 1
        }
    }
}

impl<T: BoardState> Ai for BasicAi<T> {
    type BoardState = T;

    fn next_play(
        &mut self,
        game_state: &GameState<T>,
    ) -> Result<(ValidPlay, Vec<String>), AiError> {
        if game_state.side_to_play != self.side {
            return Err(NotMyTurn);
        }
        let mut stats = SearchStats::default();
        let start_time = Instant::now();
        let (best_play, best_score) = self.iddfs(*game_state, self.side == Attacker, &mut stats);

        let log_lines: Vec<String> = vec![
            format!(
                "Searched {} paths ({} states) in {}s.",
                stats.paths,
                stats.states,
                start_time.elapsed().as_secs_f32()
            ),
            format!("Maximum depth searched: {}", stats.max_depth),
            format!("Pruned {} paths.", stats.ab_prunes),
            format!(
                "TT hits: {}, insertions: {}, replacements: {}.",
                stats.tt_hits, stats.tt_inserts, stats.tt_replacements
            ),
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
