use crate::types::{CastlingRights, Piece, Side, Square};

#[derive(Debug)]
pub struct Zobrist {
    pieces: [[u64; 64]; 12],
    side: u64,
    castling: [u64; 16],
    enpassant: [u64; 8],
    hm: [u64; 16],
}

impl Zobrist {
    pub fn piece(&self, side: Side, piece: Piece, square: Square) -> u64 {
        self.pieces[(piece as usize) + (side as usize * 6)][square]
    }

    pub fn side(&self) -> u64 {
        self.side
    }

    pub fn castling(&self, rights: CastlingRights) -> u64 {
        self.castling[rights.0 as usize]
    }

    pub fn enpassant(&self, square: Square) -> u64 {
        self.enpassant[square as usize % 8]
    }

    pub fn get_halfmove_num(&self, bucket: usize) -> u64 {
        self.hm[bucket]
    }
}

pub const fn pseudo_rand(state: &mut u64) -> u64 {
    const INCREMENT: u64 = 0x9E3779B97F4A7C15;
    const MULT1: u64 = 0xBF58476D1CE4E5B9;
    const MULT2: u64 = 0x94D049BB133111EB;

    *state = state.wrapping_add(INCREMENT);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(MULT1);
    z = (z ^ (z >> 27)).wrapping_mul(MULT2);
    z ^ (z >> 31)
}

pub const ZOBRIST: Zobrist = {
    const SEED: u64 = 0xDEE4BD7D_B659CAD9u64;
    let mut state = SEED;

    let mut pieces = [[0; 64]; 12];
    let mut x = 0;
    while x < 12 {
        let mut y = 0;
        while y < 64 {
            pieces[x][y] = pseudo_rand(&mut state);
            y += 1;
        }
        x += 1;
    }

    let side = pseudo_rand(&mut state);
    let mut castling = [0; 16];
    let mut x = 0;
    while x < 16 {
        castling[x] = pseudo_rand(&mut state);
        x += 1;
    }

    let mut enpassant = [0; 8];
    let mut x = 0;
    while x < 8 {
        enpassant[x] = pseudo_rand(&mut state);
        x += 1;
    }

    let mut hm = [0; 16];
    let mut x = 0;
    while x < 16 {
        hm[x] = pseudo_rand(&mut state);
        x += 1;
    }

    Zobrist {
        pieces,    // Number for each piece on each square 12 pieces 64 squares
        side,      // Number to indicate the side to move is black
        castling,  // Castling rights 2^4 aka all possible castling combinations.
        enpassant, // File of valid en-passant square
        hm,        // Half-move bucket
    }
};

#[cfg(test)]
mod tests {
    use crate::board::Board;
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_zobrist() {
        let zobrist = ZOBRIST;
        println!("{:?}", zobrist);

        // want to check if every number is unique
        let mut seen = HashSet::new();
        assert!(
            zobrist
                .pieces
                .iter()
                .flatten()
                .chain(zobrist.castling.iter())
                .chain(zobrist.enpassant.iter())
                .chain([zobrist.side].iter())
                .all(|e| seen.insert(e))
        );
    }

    #[test]
    fn test_board_hashing() {
        let board1 = Board::from_fen("8/6K1/3N4/8/5Q2/8/1kr5/8 w - - 0 1").unwrap();
        let ver_hash = ZOBRIST.piece(Side::White, Piece::Knight, Square::D6)
            ^ ZOBRIST.piece(Side::White, Piece::Queen, Square::F4)
            ^ ZOBRIST.piece(Side::White, Piece::King, Square::G7)
            ^ ZOBRIST.piece(Side::Black, Piece::Rook, Square::C2)
            ^ ZOBRIST.piece(Side::Black, Piece::King, Square::B2)
            ^ ZOBRIST.castling(CastlingRights(0));

        assert_eq!(board1.state.keys.full, ver_hash);
    }
}
