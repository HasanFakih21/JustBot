use crate::{
    lookup::attacks,
    types::{BitBoard, Move, MoveKind, Piece, SidedPiece, Square, ZOBRIST},
};

static mut KEYS: [u64; 8192] = [0; 8192];
static mut MOVES: [Move; 8192] = [Move::NONE; 8192];

pub fn initialize() {
    for &sided_piece in SidedPiece::ALL.iter().filter(|&&p| p.kind() != Piece::Pawn) {
        for a in 0..64 {
            for b in (a + 1)..64 {
                let a = Square::from(a);
                let b = Square::from(b);

                let piece = sided_piece.kind();
                let side = sided_piece.side();

                if !attacks(side, a, piece, BitBoard(0)).contains(b) {
                    continue;
                }

                let mut m = Move::new(a, b, MoveKind::QuietMove);
                let mut key = ZOBRIST.piece(side, piece, a) ^ ZOBRIST.piece(side, piece, b) ^ ZOBRIST.side();

                let mut slot = h1(key);

                loop {
                    unsafe {
                        std::mem::swap(&mut KEYS[slot], &mut key);
                        std::mem::swap(&mut MOVES[slot], &mut m);
                    }

                    if m == Move::NONE {
                        break;
                    }

                    slot = if slot == h1(key) { h2(key) } else { h1(key) };
                }
            }
        }
    }
}

pub const fn keys(slot: usize) -> u64 {
    unsafe { KEYS[slot] }
}

pub const fn moves(slot: usize) -> Move {
    unsafe { MOVES[slot] }
}

pub const fn h1(h: u64) -> usize {
    ((h >> 32) & 0x1FFF) as usize
}

pub const fn h2(h: u64) -> usize {
    ((h >> 48) & 0x1FFF) as usize
}
