use std::sync::LazyLock;

use crate::tools::magics::{
    BISHOP_MAGIC_NUMBERS, BISHOP_OCCUPANCY_BIT_COUNTS, ROOK_MAGIC_NUMBERS,
    ROOK_OCCUPANCY_BIT_COUNTS, magic_index, set_occupancy,
};
use crate::types::bitboard::BitBoard;
use crate::types::constants::*;
use crate::types::{Side, Square};

pub const PAWN_ATTACKS: [[BitBoard; 64]; 2] = {
    let mut pawn_attacks = [[BitBoard(0); 64]; 2];
    let mut i = 0;
    while i < 64 {
        let square = Square::from(i);
        pawn_attacks[Side::White as usize][i] = mask_pawn_attacks(Side::White, square);
        pawn_attacks[Side::Black as usize][i] = mask_pawn_attacks(Side::Black, square);
        i += 1;
    }

    pawn_attacks
};

pub const KNIGHT_ATTACKS: [BitBoard; 64] = {
    let mut knight_attacks = [BitBoard(0); 64];
    let mut i = 0;
    while i < 64 {
        let square = Square::from(i);
        knight_attacks[i] = mask_knight_attacks(square);
        i += 1;
    }

    knight_attacks
};

pub const KING_ATTACKS: [BitBoard; 64] = {
    let mut king_attacks = [BitBoard(0); 64];
    let mut i = 0;
    while i < 64 {
        let square = Square::from(i);
        king_attacks[i] = mask_king_attacks(square);
        i += 1;
    }

    king_attacks
};

pub const BISHOP_MASKS: [BitBoard; 64] = {
    let mut bishop_masks = [BitBoard(0); 64];
    let mut i = 0;
    while i < 64 {
        let square = Square::from(i);
        bishop_masks[i] = mask_bishop_attacks(square);
        i += 1;
    }

    bishop_masks
};

pub const ROOK_MASKS: [BitBoard; 64] = {
    let mut rook_masks = [BitBoard(0); 64];
    let mut i = 0;
    while i < 64 {
        let square = Square::from(i);
        rook_masks[i] = mask_rook_attacks(square);
        i += 1;
    }

    rook_masks
};

pub static BISHOP_ATTACKS: LazyLock<Vec<BitBoard>> = LazyLock::new(|| {
    let mut bishop_attacks = vec![BitBoard(0); 64 * 512];
    for square in 0..64 {
        let relevant_bits = BISHOP_OCCUPANCY_BIT_COUNTS[square];
        let magic_number = BISHOP_MAGIC_NUMBERS[square];

        for index in 0..512 {
            let occupancy_bb = set_occupancy(index, relevant_bits, BISHOP_MASKS[square]);
            let magic_index = magic_index(occupancy_bb, relevant_bits, magic_number);
            bishop_attacks[(square * 512) + magic_index] =
                blocked_bishop_attacks(Square::from(square), occupancy_bb);
        }
    }

    bishop_attacks
});

pub static ROOK_ATTACKS: LazyLock<Vec<BitBoard>> = LazyLock::new(|| {
    let mut rook_attacks = vec![BitBoard(0); 64 * 4096];
    for square in 0..64 {
        let relevant_bits = ROOK_OCCUPANCY_BIT_COUNTS[square];
        let magic_number = ROOK_MAGIC_NUMBERS[square];

        for index in 0..4096 {
            let occupancy_bb = set_occupancy(index, relevant_bits, ROOK_MASKS[square]);
            let magic_index = magic_index(occupancy_bb, relevant_bits, magic_number);
            rook_attacks[(square * 4096) + magic_index] =
                blocked_rook_attacks(Square::from(square), occupancy_bb);
        }
    }

    rook_attacks
});

pub static BETWEEN: [[BitBoard; 64]; 64] = {
    let mut between = [[BitBoard(0); 64]; 64];
    let mut square1 = 0;
    while square1 < 64 {
        let mut square2 = 0;
        while square2 < 64 {
            between[square1][square2] =
                generate_between(Square::from(square1), Square::from(square2));
            square2 += 1;
        }

        square1 += 1;
    }

    between
};

pub static RAYS: [[BitBoard; 64]; 64] = {
    let mut rays = [[BitBoard(0); 64]; 64];
    let mut square1 = 0;
    while square1 < 64 {
        let mut square2 = 0;
        while square2 < 64 {
            rays[square1][square2] = generate_ray(Square::from(square1), Square::from(square2));
            square2 += 1;
        }

        square1 += 1;
    }

    rays
};

pub const DIAGONALS: [[BitBoard; 64]; 2] = {
    let mut diagonals = [[BitBoard(0); 64]; 2];
    let mut square = 0;
    while square < 64 {
        diagonals[0][square] = BitBoard(
            generate_slide(Square::from(square), BitBoard(0), NORTH_EAST).0
                | generate_slide(Square::from(square), BitBoard(0), SOUTH_WEST).0,
        );
        diagonals[1][square] = BitBoard(
            generate_slide(Square::from(square), BitBoard(0), SOUTH_EAST).0
                | generate_slide(Square::from(square), BitBoard(0), NORTH_WEST).0,
        );

        square += 1;
    }

    diagonals
};

pub const fn generate_between(square1: Square, square2: Square) -> BitBoard {
    let directions = [
        NORTH, SOUTH, EAST, WEST, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST,
    ];
    let mut between = 0;
    let mut i = 0;

    while i < 8 {
        let direction = directions[i];
        let slide = generate_slide(square2, BitBoard(0), direction);
        if slide.contains(square1) {
            between = slide.0 & generate_slide(square1, BitBoard(0), -direction).0;
            break;
        }

        i += 1;
    }

    BitBoard(between)
}

pub const fn generate_ray(square1: Square, square2: Square) -> BitBoard {
    let directions = [
        NORTH, SOUTH, EAST, WEST, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST,
    ];
    let mut ray = 0;
    let mut i = 0;

    while i < 8 {
        let direction = directions[i];
        let slide = generate_slide(square2, BitBoard(0), direction);
        if slide.contains(square1) {
            ray = slide.0;
        }

        i += 1;
    }

    BitBoard(ray)
}

pub const fn border(direction: i8) -> BitBoard {
    let mut border = 0;
    if direction == NORTH {
        border |= RANK_8;
    }

    if direction == SOUTH {
        border |= RANK_1;
    }

    if direction == WEST {
        border |= A_FILE;
    }

    if direction == EAST {
        border |= H_FILE
    }

    if direction == NORTH_EAST {
        border |= RANK_8 | H_FILE;
    }

    if direction == NORTH_WEST {
        border |= RANK_8 | A_FILE
    }

    if direction == SOUTH_EAST {
        border |= RANK_1 | H_FILE
    }

    if direction == SOUTH_WEST {
        border |= RANK_1 | A_FILE
    }

    BitBoard(border)
}

pub const fn generate_slide(square: Square, occupancies: BitBoard, direction: i8) -> BitBoard {
    let mut slide = BitBoard(0);
    let mut step = BitBoard(1 << square as usize);
    while (step.0 & (occupancies.0 | border(direction).0)) == 0 {
        step = step.shift(direction);
        slide = BitBoard(step.0 | slide.0);
    }

    slide
}

pub const fn mask_pawn_attacks(side: Side, square: Square) -> BitBoard {
    let current = 1u64 << square as u64;
    let mut top_left = 0u64;
    let mut top_right = 0u64;

    match side {
        Side::White => {
            if current & A_FILE == 0 {
                top_left = current << 7;
            }

            if current & H_FILE == 0 {
                top_right = current << 9;
            }
        }
        Side::Black => {
            if current & H_FILE == 0 {
                top_left = current >> 7;
            }

            if current & A_FILE == 0 {
                top_right = current >> 9;
            }
        }
    }

    BitBoard(top_left | top_right)
}

pub const fn mask_knight_attacks(square: Square) -> BitBoard {
    let current = 1u64 << square as u64;

    let tl1 = if (current & A_FILE) == 0 {
        current << 15
    } else {
        0
    };
    let tl2 = if (current & (A_FILE | B_FILE)) == 0 {
        current << 6
    } else {
        0
    };

    let bl1 = if (current & (A_FILE | B_FILE)) == 0 {
        current >> 10
    } else {
        0
    };
    let bl2 = if (current & A_FILE) == 0 {
        current >> 17
    } else {
        0
    };

    let tr1 = if (current & H_FILE) == 0 {
        current << 17
    } else {
        0
    };
    let tr2 = if (current & (H_FILE | G_FILE)) == 0 {
        current << 10
    } else {
        0
    };

    let br1 = if (current & (H_FILE | G_FILE)) == 0 {
        current >> 6
    } else {
        0
    };
    let br2 = if (current & H_FILE) == 0 {
        current >> 15
    } else {
        0
    };

    BitBoard(tl1 | tl2 | bl1 | bl2 | tr1 | tr2 | br1 | br2)
}

pub const fn mask_king_attacks(square: Square) -> BitBoard {
    let current = 1u64 << square as u64;
    let n = current << 8;
    let nw = if current & A_FILE == 0 {
        current << 7
    } else {
        0
    };
    let w = if current & A_FILE == 0 {
        current >> 1
    } else {
        0
    };
    let sw = if current & A_FILE == 0 {
        current >> 9
    } else {
        0
    };
    let s = current >> 8;
    let se = if current & H_FILE == 0 {
        current >> 7
    } else {
        0
    };
    let e = if current & H_FILE == 0 {
        current << 1
    } else {
        0
    };
    let ne = if current & H_FILE == 0 {
        current << 9
    } else {
        0
    };

    BitBoard(n | nw | w | sw | s | se | e | ne)
}

pub const fn mask_bishop_attacks(square: Square) -> BitBoard {
    let mut attacks = 0u64;
    let (rank, file) = square.to_rank_and_file();

    let (mut r, mut f) = (rank, file);
    while r < 6 && f < 6 {
        r += 1;
        f += 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
    }

    let (mut r, mut f) = (rank, file);
    while r > 1 && f < 6 {
        r -= 1;
        f += 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
    }

    let (mut r, mut f) = (rank, file);
    while r < 6 && f > 1 {
        r += 1;
        f -= 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
    }

    let (mut r, mut f) = (rank, file);
    while r > 1 && f > 1 {
        r -= 1;
        f -= 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
    }

    BitBoard(attacks)
}

pub const fn mask_rook_attacks(square: Square) -> BitBoard {
    let mut attacks = 0u64;
    let (rank, file) = square.to_rank_and_file();

    let mut r = rank;
    while r > 1 {
        r -= 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, file) as u64;
    }

    let mut r = rank;
    while r < 6 {
        r += 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, file) as u64;
    }

    let mut f = file;
    while f < 6 {
        f += 1;
        attacks |= 1u64 << Square::from_rank_and_file(rank, f) as u64;
    }

    let mut f = file;
    while f > 1 {
        f -= 1;
        attacks |= 1u64 << Square::from_rank_and_file(rank, f) as u64;
    }

    BitBoard(attacks)
}

pub const fn blocked_bishop_attacks(square: Square, block_board: BitBoard) -> BitBoard {
    let mut attacks = 0u64;
    let (rank, file) = square.to_rank_and_file();

    let (mut r, mut f) = (rank, file);

    while r < 7 && f < 7 {
        r += 1;
        f += 1;

        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
        if (1u64 << Square::from_rank_and_file(r, f) as u64) & block_board.0 != 0 {
            break;
        }
    }

    let (mut r, mut f) = (rank, file);
    while r > 0 && f < 7 {
        r -= 1;
        f += 1;

        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
        if (1u64 << Square::from_rank_and_file(r, f) as u64) & block_board.0 != 0 {
            break;
        }
    }

    let (mut r, mut f) = (rank, file);
    while r < 7 && f > 0 {
        r += 1;
        f -= 1;

        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
        if (1u64 << Square::from_rank_and_file(r, f) as u64) & block_board.0 != 0 {
            break;
        }
    }

    let (mut r, mut f) = (rank, file);
    while r > 0 && f > 0 {
        r -= 1;
        f -= 1;

        attacks |= 1u64 << Square::from_rank_and_file(r, f) as u64;
        if (1u64 << Square::from_rank_and_file(r, f) as u64) & block_board.0 != 0 {
            break;
        }
    }

    BitBoard(attacks)
}

pub const fn blocked_rook_attacks(square: Square, block_board: BitBoard) -> BitBoard {
    let mut attacks = 0u64;
    let (rank, file) = square.to_rank_and_file();

    let mut r = rank;
    while r > 0 {
        r -= 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, file) as u64;
        if (1u64 << Square::from_rank_and_file(r, file) as u64) & block_board.0 != 0 {
            break;
        }
    }

    let mut r = rank;
    while r < 7 {
        r += 1;
        attacks |= 1u64 << Square::from_rank_and_file(r, file) as u64;
        if (1u64 << Square::from_rank_and_file(r, file) as u64) & block_board.0 != 0 {
            break;
        }
    }

    let mut f = file;
    while f < 7 {
        f += 1;
        attacks |= 1u64 << Square::from_rank_and_file(rank, f) as u64;
        if (1u64 << Square::from_rank_and_file(rank, f) as u64) & block_board.0 != 0 {
            break;
        }
    }

    let mut f = file;
    while f > 0 {
        f -= 1;
        attacks |= 1u64 << Square::from_rank_and_file(rank, f) as u64;
        if (1u64 << Square::from_rank_and_file(rank, f) as u64) & block_board.0 != 0 {
            break;
        }
    }

    BitBoard(attacks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::bitboard::BitBoard;

    #[test]
    fn test_pawn_attack_mask() {
        let b1 = mask_pawn_attacks(Side::White, Square::C2);
        b1.print_board();
        assert_eq!(b1, BitBoard(655360));

        let b2 = mask_pawn_attacks(Side::Black, Square::A1);
        b2.print_board();
        assert_eq!(b2, BitBoard(0));

        let b3 = mask_pawn_attacks(Side::Black, Square::A4);
        b3.print_board();
        assert_eq!(b3, BitBoard(131072));

        let b4 = mask_pawn_attacks(Side::White, Square::H3);
        b4.print_board();
        assert_eq!(b4, BitBoard(1073741824));
    }

    #[test]
    fn test_knight_attack_mask() {
        for i in 0..64 {
            let b = mask_knight_attacks(Square::from(i));
            b.print_board();
        }
    }

    #[test]
    fn test_king_attack_mask() {
        for i in 0..64 {
            let b = mask_king_attacks(Square::from(i));
            b.print_board();
        }
    }

    #[test]
    fn test_bishop_attack_mask() {
        for i in 0..64 {
            let b = mask_bishop_attacks(Square::from(i));
            b.print_board();
        }
    }

    #[test]
    fn test_rook_attack_mask() {
        for i in 0..64 {
            let b = mask_rook_attacks(Square::from(i));
            b.print_board();
        }
    }

    #[test]
    fn test_blocked_bishop_attacks() {
        let mut blocked_bitboard = BitBoard(0u64);
        blocked_bitboard.set_bit(Square::C5);
        blocked_bitboard.set_bit(Square::G5);
        blocked_bitboard.set_bit(Square::G1);
        blocked_bitboard.set_bit(Square::D2);

        blocked_bitboard.print_board();

        let b1 = blocked_bishop_attacks(Square::A3, blocked_bitboard);

        b1.print_board();
    }

    #[test]
    fn test_blocked_rook_attacks() {
        let mut blocked_bitboard = BitBoard(0u64);
        blocked_bitboard.set_bit(Square::C3);
        blocked_bitboard.set_bit(Square::H3);
        blocked_bitboard.set_bit(Square::E6);
        blocked_bitboard.set_bit(Square::E2);

        blocked_bitboard.print_board();

        let b1 = blocked_rook_attacks(Square::E3, blocked_bitboard);

        b1.print_board();
    }

    #[test]
    fn test_generate_slide() {
        let bb = generate_slide(Square::B7, BitBoard(0), SOUTH_EAST);
        bb.print_board();
    }

    #[test]
    fn test_generate_between() {
        let bb = generate_between(Square::E1, Square::E6);
        bb.print_board();
        assert_eq!(bb, BETWEEN[Square::E1 as usize][Square::E6 as usize]);
        BETWEEN[Square::E4 as usize][Square::B7 as usize].print_board();
    }

    #[test]
    fn test_diagonals() {
        let bb = DIAGONALS[Side::White as usize][Square::E4 as usize];
        bb.print_board();
    }

    #[test]
    fn test_rays() {
        let bb = RAYS[Square::E5 as usize][Square::H2 as usize];
        bb.print_board();
    }
}
