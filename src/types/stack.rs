use std::ops::{Index, IndexMut};

use crate::types::{MAX_PLY, Move, OptionPiece, PieceToHistory, Score, SidedPiece};

#[derive(Debug)]
pub struct Stack {
    data: [PlyData; MAX_PLY + 16], // Add some padding so we can start the first ply further down the array so when we do ply - index, we don't have to have any if statements,
    sentinel: PieceToHistory<i16>,
}

impl Stack {
    pub fn new() -> Box<Self> {
        let mut table = Box::new(Stack::default());
        let sentinel_ptr = &raw mut table.sentinel;
        for data in table.data.iter_mut() {
            // Gets rid of the null pointers so they instead point to an "empty" table
            data.conthistory = sentinel_ptr;
            data.contcorrhistory = sentinel_ptr;
        }

        table
    }

    pub fn sentinel(&mut self) -> *mut PieceToHistory<i16> {
        &raw mut self.sentinel
    }
}

impl Default for Stack {
    fn default() -> Self {
        Stack {
            data: [PlyData::default(); MAX_PLY + 16],
            sentinel: [[0; 64]; 13],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlyData {
    pub m: Move,
    pub piece: OptionPiece<SidedPiece>,
    pub conthistory: *mut PieceToHistory<i16>,
    pub contcorrhistory: *mut PieceToHistory<i16>,
    pub eval: i32,
    pub excluded: Move,
    pub reduction: i32,
    pub complexity: i32,
}

impl Default for PlyData {
    fn default() -> Self {
        PlyData {
            m: Move::default(),
            piece: OptionPiece::None,
            conthistory: std::ptr::null_mut(),
            contcorrhistory: std::ptr::null_mut(),
            eval: -Score::INFINITY,
            excluded: Move::default(),
            reduction: 0,
            complexity: 0,
        }
    }
}

unsafe impl Send for PlyData {}

impl Index<isize> for Stack {
    type Output = PlyData;

    fn index(&self, index: isize) -> &Self::Output {
        &self.data[(index + 8) as usize] // Allows us to check atleast 8 plies back without going out of bounds
    }
}

impl IndexMut<isize> for Stack {
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        &mut self.data[(index + 8) as usize]
    }
}
