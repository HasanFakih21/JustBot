use std::ops::{Index, IndexMut};

use crate::types::{MAX_PLY, Move, Piece, PieceToHistory, Side};

#[derive(Debug)]
pub struct PlyTable {
    data: [PlyData; MAX_PLY + 16], // Add some padding so we can start the first ply further down the array so when we do ply - index, we don't have to have any if statements,
    sentinel: PieceToHistory<i16>,
}

impl PlyTable {
    pub fn new() -> Box<Self> {
        let mut table = Box::new(PlyTable::default());
        let sentinel_ptr = &raw mut table.sentinel;
        for data in table.data.iter_mut() {
            data.conthistory = sentinel_ptr; // Gets rid of the null pointers so they instead point to an "empty" table
        }

        table
    }

    pub fn sentinel(&mut self) -> *mut PieceToHistory<i16> {
        &raw mut self.sentinel
    }
}

impl Default for PlyTable {
    fn default() -> Self {
        PlyTable {
            data: [PlyData::default(); MAX_PLY + 16],
            sentinel: [[0; 64]; 13],
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PlyData {
    pub m: Move,
    pub piece: Option<(Side, Piece)>,
    pub conthistory: *mut PieceToHistory<i16>,
    pub eval: i32,
}

unsafe impl Send for PlyData {}

impl Index<isize> for PlyTable {
    type Output = PlyData;

    fn index(&self, index: isize) -> &Self::Output {
        &self.data[(index + 8) as usize] // Allows us to check atleast 8 plies back without going out of bounds
    }
}

impl IndexMut<isize> for PlyTable {
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        &mut self.data[(index + 8) as usize]
    }
}
