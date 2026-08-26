use std::sync::LazyLock;

use crate::types::{BitBoard, Rank, Square};

pub const A_FILE: u64 = 0x0101010101010101;
pub const B_FILE: u64 = 0x0202020202020202;
pub const G_FILE: u64 = 0x4040404040404040;
pub const H_FILE: u64 = 0x8080808080808080;

pub const A: BitBoard = BitBoard(A_FILE);
pub const H: BitBoard = BitBoard(H_FILE);
pub const G: BitBoard = BitBoard(G_FILE);
pub const B: BitBoard = BitBoard(B_FILE);
pub const AB: BitBoard = BitBoard(A_FILE | B_FILE);
pub const HG: BitBoard = BitBoard(H_FILE | G_FILE);

pub const RANK_1: u64 = 0x00000000000000FF;
pub const RANK_2: u64 = 0x000000000000FF00;
pub const RANK_4: u64 = 0x00000000FF000000;
pub const RANK_5: u64 = 0x000000FF00000000;
pub const RANK_7: u64 = 0x00FF000000000000;
pub const RANK_8: u64 = 0xFF00000000000000;
pub const HOME_RANK: [Rank; 2] = [Rank::R1, Rank::R8];

pub const FULL: u64 = 0xFFFFFFFFFFFFFFFF;
pub const WK_SIDE: u64 = 0x0000000000000060;
pub const WQ_SIDE: u64 = 0x000000000000000E;

pub const BORDERS: BitBoard = BitBoard(RANK_1 | RANK_8 | A_FILE | H_FILE);

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub const NORTH: i8 = 8;
pub const SOUTH: i8 = -8;
pub const WEST: i8 = -1;
pub const EAST: i8 = 1;
pub const NORTH_WEST: i8 = 7;
pub const SOUTH_WEST: i8 = -9;
pub const SOUTH_EAST: i8 = -7;
pub const NORTH_EAST: i8 = 9;

pub const KING_SIDE_ROOK_WHITE: Square = Square::H1;
pub const QUEEN_SIDE_ROOK_WHITE: Square = Square::A1;

pub const KING_SIDE_ROOK_BLACK: Square = Square::H8;
pub const QUEEN_SIDE_ROOK_BLACK: Square = Square::A8;
pub const CASTLING_ROOK_SQAURES: [[Square; 2]; 2] = [
    [KING_SIDE_ROOK_WHITE, QUEEN_SIDE_ROOK_WHITE],
    [KING_SIDE_ROOK_BLACK, QUEEN_SIDE_ROOK_BLACK],
];

pub const ROOK_TO: [[Square; 2]; 2] = [[Square::F1, Square::D1], [Square::F8, Square::D8]];
pub const KING_TO: [[Square; 2]; 2] = [[Square::G1, Square::C1], [Square::G8, Square::C8]];

pub const MAX_PLY: usize = 248;
pub const MAX_MOVE_NUM: usize = 256;

pub const MOVE_OVERHEAD: u64 = 50;

pub const fn to_file_bb(square: Square) -> BitBoard {
    let file = square.to_file();
    BitBoard(A_FILE).shift(EAST * file as i8)
}

/// `[Is Quiet][Depth][Move Count]`
pub static LMR_TABLE: LazyLock<Box<[[[i32; 64]; 128]; 2]>> = {
    LazyLock::new(|| {
        let mut quiet_table = [[0; 64]; 128];
        let mut noisy_table = [[0; 64]; 128];

        for depth in 0..128 {
            for move_count in 0..64 {
                quiet_table[depth][move_count] = ((0.7851
                    + (move_count as f32).ln() * (depth as f32).ln() / 2.4482)
                    * 1024.0) as i32;
                noisy_table[depth][move_count] = ((0.2551
                    + (move_count as f32).ln() * (depth as f32).ln() / 3.004)
                    * 1024.0) as i32;
            }
        }

        Box::new([noisy_table, quiet_table])
    })
};
