use eframe::epaint::Color32;
use egui::{Align2, FontId, Rect, Response, Vec2};
use hnefatafl::board::state::BoardState;
use hnefatafl::game::Game;
use hnefatafl::pieces;
use hnefatafl::pieces::{Piece, PieceType, PlacedPiece};
use hnefatafl::play::{Play, PlayRecord};
use hnefatafl::tiles::{Axis, Tile};
use std::collections::{HashMap, HashSet};

struct TileColors {
    throne: Color32,
    corner: Color32,
    base_camp: Color32,
    plain: Color32,
    possible_dest: Color32,
    selected: Color32,
}

const TILE_COLORS: TileColors = TileColors {
    throne: Color32::from_gray(180),
    corner: Color32::from_gray(180),
    base_camp: Color32::from_gray(180),
    plain: Color32::from_rgb(255, 255, 240),
    selected: Color32::from_rgb(200, 240, 200),
    possible_dest: Color32::from_rgb(200, 240, 200),
};

struct Figures {
    king: char,
    white_soldier: char,
    black_soldier: char,
    up_arrow: char,
    down_arrow: char,
    left_arrow: char,
    right_arrow: char,
    captured_tile: char,
}

const FIGURES: Figures = Figures {
    king: '♔',
    white_soldier: '♙',
    black_soldier: '♟',
    up_arrow: '⬆',
    down_arrow: '⬇',
    left_arrow: '⬅',
    right_arrow: '➡',
    captured_tile: '🗙',
};

struct TileState {
    piece: Option<Piece>,
    is_throne: bool,
    is_corner: bool,
    is_base_camp: bool,
}

impl TileState {
    fn new(piece: Option<Piece>, is_throne: bool, is_corner: bool, is_base_camp: bool) -> Self {
        Self {
            piece,
            is_throne,
            is_corner,
            is_base_camp,
        }
    }
}

pub(crate) struct Board<T: BoardState> {
    /// The state of each tile.
    tile_state: HashMap<Tile, TileState>,
    /// Tiles that have been selected by the user.
    selected_tiles: (Option<Tile>, Option<Tile>),
    /// Possible destinations of the currently selected piece.
    possible_dests: HashSet<Tile>,
    /// The last play that was made.
    last_play: Option<PlayRecord<T>>,
    /// The side that the human is playing as.
    human_side: pieces::Side,
    /// The length of the board in tiles.
    board_len_tiles: u8,
}

impl<T: BoardState> Board<T> {
    pub(crate) fn new(game: &Game<T>, human_side: pieces::Side) -> Self {
        let mut tile_state: HashMap<Tile, TileState> = HashMap::new();
        for tile in game.logic.board_geo.iter_tiles() {
            tile_state.insert(
                tile,
                TileState::new(
                    game.state.board.get_piece(tile),
                    game.logic.board_geo.special_tiles.throne == tile,
                    game.logic.board_geo.special_tiles.corners.contains(tile),
                    false,
                ),
            );
        }
        Self {
            tile_state,
            selected_tiles: (None, None),
            possible_dests: HashSet::new(),
            last_play: None,
            human_side,
            board_len_tiles: game.logic.board_geo.side_len,
        }
    }
    fn update_tile_state(&mut self, board_state: T) {
        for (tile, state) in self.tile_state.iter_mut() {
            state.piece = board_state.get_piece(*tile);
        }
    }

    fn calc_tile_side_px(&self, board_side_px: f32) -> f32 {
        (board_side_px - self.board_len_tiles as f32) / (self.board_len_tiles as f32)
    }

    pub(crate) fn update(
        &mut self,
        game: &Game<T>,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        board_side_px: f32,
    ) -> Option<Play> {
        if let Some(last_play) = game.play_history.last() {
            self.last_play = Some(last_play.clone());
        }
        self.update_tile_state(game.state.board);

        let tile_len_px = self.calc_tile_side_px(board_side_px);

        let tile_size_px = Vec2::new(tile_len_px, tile_len_px);
        let mut responses: Vec<(Response, Rect, Color32, Tile)> = vec![];
        for (tile, state) in &self.tile_state {
            let color = if self.possible_dests.contains(tile) {
                TILE_COLORS.possible_dest
            } else if state.is_throne {
                TILE_COLORS.throne
            } else if state.is_corner {
                TILE_COLORS.corner
            } else if state.is_base_camp {
                TILE_COLORS.base_camp
            } else if self.selected_tiles.0 == Some(*tile) {
                TILE_COLORS.selected
            } else if self.possible_dests.contains(tile) {
                TILE_COLORS.possible_dest
            } else {
                TILE_COLORS.plain
            };
            let top_left = egui::pos2(
                (tile_len_px + 1.0) * tile.col as f32,
                (tile_len_px + 1.0) * tile.row as f32,
            );
            let bottom_right = top_left + tile_size_px;
            let rect = egui::Rect::from_two_pos(top_left, bottom_right);
            let response = ui.allocate_rect(rect, egui::Sense::click());
            responses.push((response, rect, color, *tile));
        }
        let painter = ui.painter();
        for (response, rect, color, tile) in responses {
            if response.clicked() {
                if game
                    .state
                    .board
                    .get_piece(tile)
                    .is_some_and(|p| p.side == game.state.side_to_play && p.side == self.human_side)
                {
                    // We have clicked on a tile containing our own piece and it is our turn
                    self.selected_tiles.0 = Some(tile);
                    if let Ok(iter) = game.iter_plays(tile) {
                        self.possible_dests = iter.map(|p| p.play.to()).collect();
                    };
                } else if Some(tile) == self.selected_tiles.0 {
                    // User has clicked a tile again, unselecting it.
                    self.selected_tiles.0 = None;
                    self.possible_dests = HashSet::new();
                } else if self.selected_tiles.0.is_some() && self.possible_dests.contains(&tile) {
                    // We have selected a valid destination tile.
                    self.selected_tiles.1 = Some(tile);
                }
            }
            painter.rect_filled(rect, 0.0, color);

            let fig_opt = if let Some(piece) = game.state.board.get_piece(tile) {
                Some(match piece {
                    Piece {
                        piece_type: PieceType::King,
                        side: pieces::Side::Defender,
                    } => FIGURES.king,
                    Piece {
                        piece_type: PieceType::Soldier,
                        side: pieces::Side::Defender,
                    } => FIGURES.white_soldier,
                    Piece {
                        piece_type: PieceType::Soldier,
                        side: pieces::Side::Attacker,
                    } => FIGURES.black_soldier,
                    _ => panic!("Unexpected piece type"),
                })
            } else if let Some(play_record) = &self.last_play {
                if play_record
                    .effects
                    .captures
                    .into_iter()
                    .any(|p: PlacedPiece| p.tile == tile)
                {
                    Some(FIGURES.captured_tile)
                } else if play_record.play.from == tile {
                    Some(if play_record.play.movement.axis == Axis::Vertical {
                        if play_record.play.movement.displacement > 0 {
                            FIGURES.down_arrow
                        } else {
                            FIGURES.up_arrow
                        }
                    } else if play_record.play.movement.displacement > 0 {
                        FIGURES.right_arrow
                    } else {
                        FIGURES.left_arrow
                    })
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(fig) = fig_opt {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    fig,
                    FontId::proportional(tile_len_px * 0.9),
                    Color32::BLACK,
                );
                // let img = Image::from(img_src)
                //     .rounding(5.0)
                //     .tint(Color32::LIGHT_BLUE);
                // img.paint_at(ui, rect);
            }
        }

        if game.state.side_to_play == self.human_side.other() {
            // If it's the AI's turn, we need to constantly repaint as egui won't automatically
            // detect when the AI thread has returned a play.  On native, this could be called from
            // the AI thread only when it has selected a play, but this doesn't work on web as only
            // the main thread has access to the UI.
            ctx.request_repaint();
        }

        if let (Some(from), Some(to)) = self.selected_tiles {
            // Human has made a play
            self.selected_tiles = (None, None);
            self.possible_dests = HashSet::new();
            Some(Play::from_tiles(from, to).unwrap())
        } else {
            None
        }
    }
}
