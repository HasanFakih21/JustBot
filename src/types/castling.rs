use crate::{
    board::Board,
    types::{Side, Square},
};

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Castling {
    WhiteKing = 0b0001,
    WhiteQueen = 0b0010,
    BlackKing = 0b0100,
    BlackQueen = 0b1000,
}

impl Castling {
    pub const KING_SIDE: usize = 0;
    pub const QUEEN_SIDE: usize = 1;
    pub const KINDS: [[Self; 2]; 2] = [[Self::WhiteKing, Self::WhiteQueen], [Self::BlackKing, Self::BlackQueen]];

    pub const fn from(c: char) -> Self {
        match c {
            'K' => Castling::WhiteKing,
            'k' => Castling::BlackKing,
            'Q' => Castling::WhiteQueen,
            'q' => Castling::BlackQueen,
            _ => panic!("Invalid character for castling identifier!"),
        }
    }

    pub const fn to_char(&self) -> char {
        match self {
            Self::WhiteKing => 'K',
            Self::BlackKing => 'k',
            Self::WhiteQueen => 'Q',
            Self::BlackQueen => 'q',
        }
    }

    pub const fn king_landing_square(&self) -> Square {
        match self {
            Self::WhiteKing => Square::G1,
            Self::WhiteQueen => Square::C1,
            Self::BlackKing => Square::G8,
            Self::BlackQueen => Square::C8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub fn new() -> Self {
        CastlingRights(0)
    }

    pub const fn can(&self, kind: Castling) -> bool {
        (self.0 & kind as u8) != 0
    }

    pub const fn can_king_side(&self, side: Side) -> bool {
        match side {
            Side::White => (Castling::WhiteKing as u8 & self.0) != 0,
            Side::Black => (Castling::BlackKing as u8 & self.0) != 0,
        }
    }

    pub const fn can_queen_side(&self, side: Side) -> bool {
        match side {
            Side::White => (Castling::WhiteQueen as u8 & self.0) != 0,
            Side::Black => (Castling::BlackQueen as u8 & self.0) != 0,
        }
    }

    pub fn set_king_side(&mut self, side: Side) {
        match side {
            Side::White => self.0 |= Castling::WhiteKing as u8,
            Side::Black => self.0 |= Castling::BlackKing as u8,
        }
    }

    pub fn set_queen_side(&mut self, side: Side) {
        match side {
            Side::White => self.0 |= Castling::WhiteQueen as u8,
            Side::Black => self.0 |= Castling::BlackQueen as u8,
        }
    }

    pub fn set(&mut self, mask: u8) {
        self.0 |= mask;
    }

    pub fn clear_king_side(&mut self, side: Side) {
        match side {
            Side::White => {
                if self.can_king_side(side) {
                    self.0 ^= Castling::WhiteKing as u8
                }
            }
            Side::Black => {
                if self.can_king_side(side) {
                    self.0 ^= Castling::BlackKing as u8
                }
            }
        }
    }

    pub fn clear(&mut self, kind: Castling) {
        if self.can(kind) {
            self.0 ^= kind as u8
        }
    }

    pub fn clear_queen_side(&mut self, side: Side) {
        match side {
            Side::White => {
                if self.can_queen_side(side) {
                    self.0 ^= Castling::WhiteQueen as u8
                }
            }
            Side::Black => {
                if self.can_queen_side(side) {
                    self.0 ^= Castling::BlackQueen as u8
                }
            }
        }
    }

    pub fn to_string(&self, board: &Board) -> String {
        let mut output_string = String::from("");

        if board.frc {
            if self.can_king_side(Side::White) {
                let file = b'A' + board.castling_rooks[Side::White][Castling::KING_SIDE].to_file() as u8;
                output_string.push(file as char);
            }
            if self.can_queen_side(Side::White) {
                let file = b'A' + board.castling_rooks[Side::White][Castling::QUEEN_SIDE].to_file() as u8;
                output_string.push(file as char);
            }
            if self.can_king_side(Side::Black) {
                let file = b'a' + board.castling_rooks[Side::Black][Castling::KING_SIDE].to_file() as u8;
                output_string.push(file as char);
            }
            if self.can_queen_side(Side::Black) {
                let file = b'a' + board.castling_rooks[Side::Black][Castling::QUEEN_SIDE].to_file() as u8;
                output_string.push(file as char);
            }
        } else {
            if self.can_king_side(Side::White) {
                output_string.push('K');
            }
            if self.can_queen_side(Side::White) {
                output_string.push('Q');
            }
            if self.can_king_side(Side::Black) {
                output_string.push('k');
            }
            if self.can_queen_side(Side::Black) {
                output_string.push('q');
            }
        }

        if output_string.is_empty() {
            output_string.push('-');
        }

        output_string
    }
}

impl Default for CastlingRights {
    fn default() -> Self {
        CastlingRights::new()
    }
}
