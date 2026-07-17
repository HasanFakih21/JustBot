use std::fmt::Display;

use crate::types::{BitBoard, Castling, Piece, Square, stackvec::StackVec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveEntry {
    pub mv: Move,
    pub score: i32,
}

#[derive(Debug, Clone)]
pub struct MoveList {
    inner: StackVec<MoveEntry, 256>,
}

#[derive(Debug, Clone)]
pub struct MoveListError;

impl MoveList {
    pub fn new() -> Self {
        MoveList {
            inner: StackVec::new(),
        }
    }

    pub fn push(&mut self, m: Move) {
        self.inner.push(MoveEntry { mv: m, score: 0 });
    }

    pub fn push_entry(&mut self, e: MoveEntry) {
        self.inner.push(e);
    }

    pub fn remove(&mut self, index: usize) -> Option<MoveEntry> {
        self.inner.remove(index)
    }

    pub fn get(&self, index: usize) -> MoveEntry {
        self.inner.get(index)
    }

    pub fn push_promotion_captures_setwise(&mut self, offset: i8, targets: BitBoard) {
        self.push_pawn_moves_setwise(offset, targets, MoveKind::QPromCapture);
        self.push_pawn_moves_setwise(offset, targets, MoveKind::RPromCapture);
        self.push_pawn_moves_setwise(offset, targets, MoveKind::NPromCapture);
        self.push_pawn_moves_setwise(offset, targets, MoveKind::BPromCapture);
    }

    pub fn push_pawn_moves_setwise(&mut self, offset: i8, targets: BitBoard, kind: MoveKind) {
        for target in targets.iter() {
            self.push(Move::new(target.shift(-offset).unwrap(), target, kind));
        }
    }

    pub fn push_setwise(&mut self, from: Square, to_bb: BitBoard, kind: MoveKind) {
        for to in to_bb.iter() {
            self.push(Move::new(from, to, kind));
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn pop(&mut self) -> Option<MoveEntry> {
        self.inner.pop()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MoveEntry> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MoveEntry> {
        self.inner.iter_mut()
    }
}

impl FromIterator<Move> for MoveList {
    fn from_iter<T: IntoIterator<Item = Move>>(iter: T) -> Self {
        let mut ml = MoveList::new();
        for e in iter {
            ml.push(e);
        }

        ml
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for MoveList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::new();
        for m in self.iter() {
            output = format!("{output}{} ", m.mv);
        }

        write!(f, "{output}")
    }
}

//12 bits for to and from square and 4 bits for move type
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(u16);

impl Move {
    pub fn new(from: Square, to: Square, kind: MoveKind) -> Self {
        Move(from as u16 | ((to as u16) << 6) | ((kind as u16) << 12))
    }

    pub const fn get_capture_square(&self) -> Square {
        if self.is_en_passant() {
            Square::from(self.get_to() as usize ^ 8)
        } else {
            self.get_to()
        }
    }

    pub const fn get_from(&self) -> Square {
        Square::from((0x003F & self.0) as usize)
    }

    pub const fn get_to(&self) -> Square {
        Square::from(((0x0FC0 & self.0) >> 6) as usize)
    }

    pub const fn get_kind(&self) -> MoveKind {
        MoveKind::from(((0xF000 & self.0) >> 12) as u8)
    }

    pub const fn is_castling(&self) -> bool {
        matches!(
            self.get_kind(),
            MoveKind::KingCastle | MoveKind::QueenCastle
        )
    }

    pub const fn is_promotion(&self) -> bool {
        self.get_kind().is_promotion()
    }

    pub const fn is_capture(&self) -> bool {
        self.get_kind().is_capture()
    }

    pub const fn is_en_passant(&self) -> bool {
        matches!(self.get_kind(), MoveKind::EnPassant)
    }

    pub const fn get_promoted_piece(&self) -> Option<Piece> {
        let kind = self.get_kind();
        let mut promoted_piece = None;

        if kind.is_knight_promotion() {
            promoted_piece = Some(Piece::Knight)
        } else if kind.is_bishop_promotion() {
            promoted_piece = Some(Piece::Bishop)
        } else if kind.is_rook_promotion() {
            promoted_piece = Some(Piece::Rook)
        } else if kind.is_queen_promotion() {
            promoted_piece = Some(Piece::Queen)
        }
        promoted_piece
    }

    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    pub const fn castle_kind(&self) -> Option<usize> {
        match self.get_kind() {
            MoveKind::KingCastle => Some(Castling::KING_SIDE),
            MoveKind::QueenCastle => Some(Castling::QUEEN_SIDE),
            _ => None,
        }
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            return write!(f, "0000");
        }

        let mut promotion_piece = "";
        match self.get_kind() {
            MoveKind::BPromotion | MoveKind::BPromCapture => promotion_piece = "b",
            MoveKind::NPromotion | MoveKind::NPromCapture => promotion_piece = "n",
            MoveKind::RPromotion | MoveKind::RPromCapture => promotion_piece = "r",
            MoveKind::QPromotion | MoveKind::QPromCapture => promotion_piece = "q",
            _ => (),
        }
        write!(f, "{}{}{}", self.get_from(), self.get_to(), promotion_piece)
    }
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveKind {
    QuietMove    = 0b0000,
    DoublePawn   = 0b0001,
    KingCastle   = 0b0010,
    QueenCastle  = 0b0011,
    Capture      = 0b0100,
    EnPassant    = 0b0101,
    NPromotion   = 0b1000,
    BPromotion   = 0b1001,
    RPromotion   = 0b1010,
    QPromotion   = 0b1011,
    NPromCapture = 0b1100,
    BPromCapture = 0b1101,
    RPromCapture = 0b1110,
    QPromCapture = 0b1111,
}

impl MoveKind {
    pub const fn from(value: u8) -> Self {
        use MoveKind::*;
        match value {
            0b0000 => QuietMove,
            0b0001 => DoublePawn,
            0b0010 => KingCastle,
            0b0011 => QueenCastle,
            0b0100 => Capture,
            0b0101 => EnPassant,
            0b1000 => NPromotion,
            0b1001 => BPromotion,
            0b1010 => RPromotion,
            0b1011 => QPromotion,
            0b1100 => NPromCapture,
            0b1101 => BPromCapture,
            0b1110 => RPromCapture,
            0b1111 => QPromCapture,
            _ => panic!("Not a valid move kind!!"),
        }
    }

    pub const fn is_quiet(&self) -> bool {
        use MoveKind::*;
        matches!(self, QuietMove | DoublePawn | KingCastle | QueenCastle)
    }

    pub const fn is_promotion(&self) -> bool {
        use MoveKind::*;
        matches!(
            self,
            NPromotion
                | BPromotion
                | RPromotion
                | QPromotion
                | NPromCapture
                | BPromCapture
                | RPromCapture
                | QPromCapture
        )
    }

    pub const fn is_knight_promotion(&self) -> bool {
        use MoveKind::*;
        matches!(self, NPromCapture | NPromotion)
    }

    pub const fn is_bishop_promotion(&self) -> bool {
        use MoveKind::*;
        matches!(self, BPromCapture | BPromotion)
    }

    pub const fn is_rook_promotion(&self) -> bool {
        use MoveKind::*;
        matches!(self, RPromCapture | RPromotion)
    }

    pub const fn is_queen_promotion(&self) -> bool {
        use MoveKind::*;
        matches!(self, QPromCapture | QPromotion)
    }

    pub const fn is_capture(&self) -> bool {
        use MoveKind::*;
        matches!(
            self,
            Capture | NPromCapture | BPromCapture | RPromCapture | QPromCapture | EnPassant
        )
    }
}
