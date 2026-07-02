use std::sync::LazyLock;

use crate::types::{BitBoard, Piece, Side, Square};

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

pub const INFINITY: i32 = 100000;
pub const MATE_SCORE: i32 = 9000;
pub const MATE_CUTOFF: i32 = 8900;
pub const TIMEOUT_SCORE: i32 = 111111;

pub const MAX_PLY: u8 = 128;
pub const MAX_HISTORY: i32 = 8000;
pub const MAX_MOVE_NUM: usize = 256;

pub const fn to_file_bb(square: Square) -> BitBoard {
    let file = square.to_file();
    BitBoard(A_FILE).shift(EAST * file as i8)
}

pub const fn to_piece_index(piece: Option<(Side, Piece)>) -> usize {
    match piece {
        Some((s, p)) => (s as usize * 6) + p as usize,
        None => 12,
    }
}

/// `[Is Quiet][Depth][Move Count]`
pub static LMR_TABLE: LazyLock<Box<[[[i32; MAX_MOVE_NUM]; MAX_PLY as usize]; 2]>> = {
    LazyLock::new(|| {
        let mut quiet_table = [[0; MAX_MOVE_NUM]; MAX_PLY as usize];
        let mut noisy_table = [[0; MAX_MOVE_NUM]; MAX_PLY as usize];

        for depth in 0..MAX_PLY {
            for move_count in 0..MAX_MOVE_NUM {
                let reduction = 0.7844 + f32::ln(depth as f32) * f32::ln(move_count as f32);

                quiet_table[depth as usize][move_count] = ((reduction / 2.4696) * 1024.0) as i32;
                noisy_table[depth as usize][move_count] = ((reduction / 3.0) * 1024.0) as i32;
            }
        }

        Box::new([noisy_table, quiet_table])
    })
};
