use crate::{
    attacks::{
        BISHOP_ATTACKS, BISHOP_MASKS, KING_ATTACKS, KNIGHT_ATTACKS, PAWN_ATTACKS, ROOK_ATTACKS,
        ROOK_MASKS,
    },
    tools::magics::{
        BISHOP_MAGIC_NUMBERS, BISHOP_OCCUPANCY_BIT_COUNTS, ROOK_MAGIC_NUMBERS,
        ROOK_OCCUPANCY_BIT_COUNTS, magic_index,
    },
    types::{BitBoard, Piece, Side, Square},
};

pub fn attacks(side: Side, square: Square, piece: Piece, occunpancies: BitBoard) -> BitBoard {
    match piece {
        Piece::Pawn => pawn_attacks(square, side),
        Piece::Knight => knight_attacks(square),
        Piece::Bishop => bishop_attacks(square, occunpancies),
        Piece::Rook => rook_attacks(square, occunpancies),
        Piece::Queen => queen_attacks(square, occunpancies),
        Piece::King => king_attacks(square),
    }
}

pub const fn pawn_attacks(square: Square, side: Side) -> BitBoard {
    PAWN_ATTACKS[side as usize][square as usize]
}

pub const fn knight_attacks(square: Square) -> BitBoard {
    KNIGHT_ATTACKS[square as usize]
}

pub const fn king_attacks(square: Square) -> BitBoard {
    KING_ATTACKS[square as usize]
}

pub fn bishop_attacks(square: Square, board_occupancy: BitBoard) -> BitBoard {
    let occupancy = board_occupancy & BISHOP_MASKS[square as usize];
    let magic_index = magic_index(
        occupancy,
        BISHOP_OCCUPANCY_BIT_COUNTS[square as usize],
        BISHOP_MAGIC_NUMBERS[square as usize],
    );

    let offset = (square as usize * 512) + magic_index;

    BISHOP_ATTACKS[offset]
}

pub fn rook_attacks(square: Square, board_occupancy: BitBoard) -> BitBoard {
    let occupancy = board_occupancy & ROOK_MASKS[square as usize];
    let magic_index = magic_index(
        occupancy,
        ROOK_OCCUPANCY_BIT_COUNTS[square as usize],
        ROOK_MAGIC_NUMBERS[square as usize],
    );

    let offset = (square as usize * 4096) + magic_index;

    ROOK_ATTACKS[offset]
}

pub fn queen_attacks(square: Square, board_occupancy: BitBoard) -> BitBoard {
    bishop_attacks(square, board_occupancy) | rook_attacks(square, board_occupancy)
}
