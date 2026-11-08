use crate::{
    lookup::attacks,
    types::{BitBoard, Piece, SidedPiece, Square, ZOBRIST},
};

static mut CUCKOO: [u64; 8192] = [0; 8192];
static mut A: [Option<Square>; 8192] = [None; 8192];
static mut B: [Option<Square>; 8192] = [None; 8192];

pub fn initialize() {
    for &sided_piece in SidedPiece::ALL.iter().filter(|&&p| p.kind() != Piece::Pawn) {
        for a in 0..64 {
            for b in (a + 1)..64 {
                let mut a = Some(Square::from(a));
                let mut b = Some(Square::from(b));

                let piece = sided_piece.kind();
                let side = sided_piece.side();

                if !attacks(side, a.unwrap(), piece, BitBoard(0)).contains(b.unwrap()) {
                    continue;
                }

                let mut m =
                    ZOBRIST.piece(side, piece, a.unwrap()) ^ ZOBRIST.piece(side, piece, b.unwrap()) ^ ZOBRIST.side();
                let mut i = h1(m);

                loop {
                    unsafe {
                        std::mem::swap(&mut CUCKOO[i], &mut m);
                        std::mem::swap(&mut A[i], &mut a);
                        std::mem::swap(&mut B[i], &mut b);
                    }

                    if a.is_none() && b.is_none() {
                        break;
                    }

                    i = if i == h1(m) { h2(m) } else { h1(m) };
                }
            }
        }
    }
}

pub const fn h1(h: u64) -> usize {
    ((h >> 32) & 0x1FFF) as usize
}

pub const fn h2(h: u64) -> usize {
    ((h >> 48) & 0x1FFF) as usize
}

pub fn cuckoo(index: usize) -> u64 {
    unsafe { CUCKOO[index] }
}

pub fn cuckoo_a(index: usize) -> Square {
    unsafe { A[index].unwrap_unchecked() }
}

pub fn cuckoo_b(index: usize) -> Square {
    unsafe { B[index].unwrap_unchecked() }
}
