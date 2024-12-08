use std::collections::{HashMap, HashSet};
use eframe::epaint::Color32;
use egui::{include_image, ImageSource, Rect, Response, Vec2};
use hnefatafl::board::state::BoardState;
use hnefatafl::game::Game;
use hnefatafl::pieces;
use hnefatafl::pieces::{Piece, PieceType};
use hnefatafl::play::{Play, PlayRecord};
use hnefatafl::tiles::{Axis, Tile};

const TILE_LENGTH: f32 = 32.0;

struct TileColors {
    throne: Color32,
    corner: Color32,
    base_camp: Color32,
    plain: Color32,
    possible_dest: Color32,
    selected: Color32
}

const TILE_COLORS: TileColors = TileColors {
    throne: Color32::from_gray(180),
    corner: Color32::from_gray(180),
    base_camp: Color32::from_gray(180),
    plain: Color32::from_gray(240),
    selected: Color32::from_rgb(200, 240, 200),
    possible_dest: Color32::from_rgb(200, 240, 200)
};

struct ImageSources<'a> {
    king: ImageSource<'a>,
    white_soldier: ImageSource<'a>,
    black_soldier: ImageSource<'a>,
    up_arrow: ImageSource<'a>,
    down_arrow: ImageSource<'a>,
    left_arrow: ImageSource<'a>,
    right_arrow: ImageSource<'a>,
    captured_tile: ImageSource<'a>
}

const IMAGES: ImageSources = ImageSources {
    king: include_image!("../res/assets/icons/king-white.svg"),
    white_soldier: include_image!("../res/assets/icons/pawn-white.svg"),
    black_soldier: include_image!("../res/assets/icons/pawn-black.svg"),
    up_arrow: include_image!("../res/assets/icons/arrow-up.svg"),
    down_arrow: include_image!("../res/assets/icons/arrow-down.svg"),
    left_arrow: include_image!("../res/assets/icons/arrow-left.svg"),
    right_arrow: include_image!("../res/assets/icons/arrow-right.svg"),
    captured_tile: include_image!("../res/assets/icons/x.svg")
};

struct TileState {
    piece: Option<Piece>,
    is_throne: bool,
    is_corner: bool,
    is_base_camp: bool
}

impl TileState {
    fn new(
        piece: Option<Piece>,
        is_throne: bool,
        is_corner: bool,
        is_base_camp: bool
    ) -> Self {
        Self {
            piece,
            is_throne,
            is_corner,
            is_base_camp
        }
    }
}


pub(crate) struct Board {
    tile_state: HashMap<Tile, TileState>,
    selected_tiles: (Option<Tile>, Option<Tile>),
    possible_dests: HashSet<Tile>,
    last_play: Option<PlayRecord>,
    player_side: pieces::Side
}

impl Board {

    pub(crate) fn new<T: BoardState>(game: &Game<T>, player_side: pieces::Side) -> Self {
        let mut tile_state: HashMap<Tile, TileState> = HashMap::new();
        for tile in game.logic.board_geo.iter_tiles() {
            tile_state.insert(tile, TileState::new(
                game.state.board.get_piece(tile),
                game.logic.board_geo.special_tiles.throne == tile,
                game.logic.board_geo.special_tiles.corners.contains(&tile),
                false
            ));
        }
        Self {
            tile_state,
            selected_tiles: (None, None),
            possible_dests: HashSet::new(),
            last_play: None,
            player_side
        }
    }
    fn update_tile_state<T: BoardState>(&mut self, board_state: T) {
        for (tile, state) in self.tile_state.iter_mut() {
            state.piece = board_state.get_piece(*tile);
        }
    }

    pub(crate) fn update<T: BoardState>(&mut self, game: &Game<T>, ctx: &egui::Context, ui: &mut egui::Ui) -> Option<Play> {
        if let Some(last_play) = game.play_history.last() {
            self.last_play = Some(last_play.clone());
        }
        self.update_tile_state(game.state.board);

        let tile_size = Vec2::new(TILE_LENGTH, TILE_LENGTH);
        let mut responses: Vec<(Response, Rect, Color32, Tile)> = vec![];
        for (tile, state) in &self.tile_state {
            let color = if self.possible_dests.contains(&tile) {
                TILE_COLORS.possible_dest
            } else if state.is_throne {
                TILE_COLORS.throne
            } else if state.is_corner {
                TILE_COLORS.corner
            } else if state.is_base_camp {
                TILE_COLORS.base_camp
            } else if self.selected_tiles.0 == Some(*tile) {
                TILE_COLORS.selected
            } else if self.possible_dests.contains(&tile) {
                TILE_COLORS.possible_dest
            } else {
                TILE_COLORS.plain
            };
            let top_left = egui::pos2(
                (TILE_LENGTH + 1.0) * tile.col as f32,
                (TILE_LENGTH + 1.0) * tile.row as f32
            );
            let bottom_right = top_left + tile_size;
            let rect = egui::Rect::from_two_pos(top_left, bottom_right);
            let response = ui.allocate_rect(rect, egui::Sense::click());
            responses.push((response, rect, color, *tile));
        }
        let painter = ui.painter();
        for (response, rect, color, tile) in responses {
            if response.clicked() {
                if game.state.board.get_piece(tile).is_some_and(|p|
                    p.side == game.state.side_to_play && p.side == self.player_side
                ) {
                    // We have clicked on a tile containing our own piece and it is our turn
                    self.selected_tiles.0 = Some(tile);
                    if let Ok(iter) = game.iter_plays(tile) {
                        self.possible_dests = iter.map(|p| p.to()).collect();
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

            let img = if let Some(piece) = game.state.board.get_piece(tile) {
                Some(match piece {
                    Piece {piece_type: PieceType::King, side: pieces::Side::Defender} => IMAGES.king,
                    Piece {piece_type: PieceType::Soldier, side: pieces::Side::Defender} => IMAGES.white_soldier,
                    Piece {piece_type: PieceType::Soldier, side: pieces::Side::Attacker} => IMAGES.black_soldier,
                    _ => panic!("Unexpected piece type")
                })
            } else if let Some(play_record) = &self.last_play {
                if play_record.outcome.captures.iter().any(|p| p.tile == tile) {
                    Some(IMAGES.captured_tile)
                } else if play_record.play.from == tile {
                    Some(if play_record.play.movement.axis == Axis::Vertical {
                        if play_record.play.movement.displacement > 0 {
                            IMAGES.down_arrow
                        } else {
                            IMAGES.up_arrow
                        }
                    } else {
                        if play_record.play.movement.displacement > 0 {
                            IMAGES.right_arrow
                        } else {
                            IMAGES.left_arrow
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(img) = img {
                egui::Image::new(img)
                    .rounding(5.0)
                    .tint(Color32::LIGHT_BLUE)
                    .paint_at(ui, rect);

            }
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