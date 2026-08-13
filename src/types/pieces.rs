use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use crate::types::Side;

#[derive(Debug)]
pub struct InvalidPiece;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl TryFrom<usize> for Piece {
    type Error = InvalidPiece;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value <= 5 {
            Ok(unsafe { std::mem::transmute::<u8, Piece>(value as u8) })
        } else {
            Err(InvalidPiece)
        }
    }
}

impl Piece {
    pub const NUM: usize = 6;
    pub const ALL: [Self; Self::NUM] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];

    pub const fn from(value: usize) -> Self {
        debug_assert!(value < 6);
        unsafe { std::mem::transmute(value as u8) }
    }

    pub const fn from_char(value: char) -> Result<Self, InvalidPiece> {
        match value.to_ascii_uppercase() {
            'P' => Ok(Piece::Pawn),
            'N' => Ok(Piece::Knight),
            'B' => Ok(Piece::Bishop),
            'R' => Ok(Piece::Rook),
            'Q' => Ok(Piece::Queen),
            'K' => Ok(Piece::King),
            _ => Err(InvalidPiece),
        }
    }

    pub const fn value(&self) -> i32 {
        match self {
            Self::Pawn => 100,
            Self::Knight => 320,
            Self::Bishop => 330,
            Self::Rook => 500,
            Self::Queen => 900,
            Self::King => 0,
        }
    }

    pub fn to_char(&self, side: Side) -> char {
        let mut c = self.to_string().chars().last().unwrap();
        if side == Side::Black {
            c.make_ascii_lowercase();
        }

        c
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            Piece::Pawn => "P",
            Piece::Knight => "N",
            Piece::Bishop => "B",
            Piece::Rook => "R",
            Piece::Queen => "Q",
            Piece::King => "K",
        };

        write!(f, "{output}")
    }
}

impl<T> Index<Piece> for [T] {
    type Output = T;

    fn index(&self, index: Piece) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Piece> for [T] {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum SidedPiece {
    WhitePawn,
    WhiteKnight,
    WhiteBishop,
    WhiteRook,
    WhiteQueen,
    WhiteKing,
    BlackPawn,
    BlackKnight,
    BlackBishop,
    BlackRook,
    BlackQueen,
    BlackKing,
}

impl SidedPiece {
    pub const fn from(side: Side, piece: Piece) -> Self {
        unsafe { std::mem::transmute(piece as u8 + (6 * side as u8)) }
    }

    pub const fn kind(&self) -> Piece {
        unsafe { std::mem::transmute(*self as u8 % 6) }
    }

    pub const fn side(&self) -> Side {
        unsafe { std::mem::transmute(*self as u8 > 5) }
    }

    pub const fn to_char(self) -> char {
        match self {
            SidedPiece::WhitePawn => 'P',
            SidedPiece::WhiteKnight => 'N',
            SidedPiece::WhiteBishop => 'B',
            SidedPiece::WhiteRook => 'R',
            SidedPiece::WhiteQueen => 'Q',
            SidedPiece::WhiteKing => 'K',
            SidedPiece::BlackPawn => 'p',
            SidedPiece::BlackKnight => 'n',
            SidedPiece::BlackBishop => 'b',
            SidedPiece::BlackRook => 'r',
            SidedPiece::BlackQueen => 'q',
            SidedPiece::BlackKing => 'k',
        }
    }
}

impl Display for SidedPiece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

pub trait PieceRepresentation {
    const NONE_INDEX: usize;

    fn as_usize(&self) -> usize;
}

impl PieceRepresentation for SidedPiece {
    const NONE_INDEX: usize = 12;

    fn as_usize(&self) -> usize {
        *self as usize
    }
}

impl PieceRepresentation for Piece {
    const NONE_INDEX: usize = 6;

    fn as_usize(&self) -> usize {
        *self as usize
    }
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub enum OptionPiece<T: PieceRepresentation> {
    #[default]
    None,
    Some(T),
}

impl<T: PieceRepresentation> OptionPiece<T> {
    pub fn unwrap(self) -> T {
        match self {
            OptionPiece::Some(piece) => piece,
            OptionPiece::None => panic!("Can't unwrap 'None' piece!"),
        }
    }

    pub fn map<U, F>(self, f: F) -> OptionPiece<U>
    where
        F: FnOnce(T) -> U,
        U: PieceRepresentation,
    {
        match self {
            OptionPiece::Some(x) => OptionPiece::Some(f(x)),
            OptionPiece::None => OptionPiece::None,
        }
    }
}

impl<T, P> Index<OptionPiece<P>> for [T]
where
    P: PieceRepresentation,
{
    type Output = T;

    fn index(&self, index: OptionPiece<P>) -> &Self::Output {
        &self[{
            match index {
                OptionPiece::Some(piece) => piece.as_usize(),
                OptionPiece::None => P::NONE_INDEX,
            }
        }]
    }
}

impl<T, P> IndexMut<OptionPiece<P>> for [T]
where
    P: PieceRepresentation,
{
    fn index_mut(&mut self, index: OptionPiece<P>) -> &mut Self::Output {
        &mut self[{
            match index {
                OptionPiece::Some(piece) => piece.as_usize(),
                OptionPiece::None => P::NONE_INDEX,
            }
        }]
    }
}
