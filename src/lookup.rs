use crate::{
    attacks::{BISHOP_ATTACKS, BISHOP_MASKS, KING_ATTACKS, KNIGHT_ATTACKS, PAWN_ATTACKS, ROOK_ATTACKS, ROOK_MASKS},
    tools::magics::{
        BISHOP_MAGIC_NUMBERS, BISHOP_OCCUPANCY_BIT_COUNTS, ROOK_MAGIC_NUMBERS, ROOK_OCCUPANCY_BIT_COUNTS, magic_index,
    },
    types::{BitBoard, Piece, Side, Square},
};

pub fn attacks(side: Side, square: Square, piece: Piece, occupancies: BitBoard) -> BitBoard {
    match piece {
        Piece::Pawn => pawn_attacks(square, side),
        Piece::Knight => knight_attacks(square),
        Piece::Bishop => bishop_attacks(square, occupancies),
        Piece::Rook => rook_attacks(square, occupancies),
        Piece::Queen => queen_attacks(square, occupancies),
        Piece::King => king_attacks(square),
    }
}

pub fn pawn_attacks(square: Square, side: Side) -> BitBoard {
    unsafe { *PAWN_ATTACKS.get_unchecked(side as usize).get_unchecked(square as usize) }
}

pub fn knight_attacks(square: Square) -> BitBoard {
    unsafe { *KNIGHT_ATTACKS.get_unchecked(square as usize) }
}

pub fn king_attacks(square: Square) -> BitBoard {
    unsafe { *KING_ATTACKS.get_unchecked(square as usize) }
}

pub fn bishop_attacks(square: Square, board_occupancy: BitBoard) -> BitBoard {
    let occupancy = board_occupancy & unsafe { *BISHOP_MASKS.get_unchecked(square as usize) };
    let magic_index = magic_index(
        occupancy,
        unsafe { *BISHOP_OCCUPANCY_BIT_COUNTS.get_unchecked(square as usize) },
        unsafe { *BISHOP_MAGIC_NUMBERS.get_unchecked(square as usize) },
    );

    let offset = (square as usize * 512) + magic_index;

    unsafe { *BISHOP_ATTACKS.get_unchecked(offset) }
}

pub fn rook_attacks(square: Square, board_occupancy: BitBoard) -> BitBoard {
    let occupancy = board_occupancy & unsafe { *ROOK_MASKS.get_unchecked(square as usize) };
    let magic_index = magic_index(
        occupancy,
        unsafe { *ROOK_OCCUPANCY_BIT_COUNTS.get_unchecked(square as usize) },
        unsafe { *ROOK_MAGIC_NUMBERS.get_unchecked(square as usize) },
    );

    let offset = (square as usize * 4096) + magic_index;

    unsafe { *ROOK_ATTACKS.get_unchecked(offset) }
}

pub fn queen_attacks(square: Square, board_occupancy: BitBoard) -> BitBoard {
    bishop_attacks(square, board_occupancy) | rook_attacks(square, board_occupancy)
}
