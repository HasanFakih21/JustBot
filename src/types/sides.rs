use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub const NUM: usize = 2;
    pub const ALL: [Self; Self::NUM] = [Self::White, Self::Black];

    pub const fn other(&self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::White => write!(f, "w"),
            Side::Black => write!(f, "b"),
        }
    }
}

impl<T> Index<Side> for [T] {
    type Output = T;

    fn index(&self, index: Side) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Side> for [T] {
    fn index_mut(&mut self, index: Side) -> &mut Self::Output {
        &mut self[index as usize]
    }
}
