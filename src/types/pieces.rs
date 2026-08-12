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
            Self::Pawn => 102,
            Self::Knight => 320,
            Self::Bishop => 328,
            Self::Rook => 505,
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
