use crate::types::{CastlingRights, Piece, Side, Square, ZOBRIST};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Keys {
    pub full: u64,
    pub pawn: u64,
    pub non_pawn: [u64; 2],
}

impl Keys {
    pub fn toggle(&mut self, side: Side, piece: Piece, square: Square) {
        let key = ZOBRIST.get_piece_num(side, piece, square);

        self.full ^= key;
        if piece == Piece::Pawn {
            self.pawn ^= key;
        } else {
            self.non_pawn[side as usize] ^= key;
        }
    }

    pub fn toggle_castling(&mut self, rights: CastlingRights) {
        self.full ^= ZOBRIST.get_castling_num(rights)
    }

    pub fn toggle_en_passant(&mut self, square: Square) {
        self.full ^= ZOBRIST.get_enpassant_num(square)
    }

    pub fn toggle_side(&mut self) {
        self.full ^= ZOBRIST.get_side_num()
    }
}
