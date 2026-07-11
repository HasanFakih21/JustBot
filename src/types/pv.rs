use crate::types::{MAX_PLY, Move};

#[derive(Debug)]
pub struct PVTable {
    inner: Box<[[Move; MAX_PLY + 1]]>,
    len: [usize; MAX_PLY + 1],
}

impl PVTable {
    pub fn new() -> Self {
        PVTable {
            inner: Box::new([[Move::default(); MAX_PLY + 1]; MAX_PLY + 1]), 
            len: [0; MAX_PLY + 1],
        }
    }

    pub fn line(&self) -> &[Move] {
        &self.inner[0][..self.len[0]]
    }

    pub fn add(&mut self, m: Move, ply: isize) {
        self.inner[ply as usize][0] = m;
        self.len[ply as usize] = self.len[(ply + 1) as usize] + 1;

        for index in 0..self.len[(ply + 1) as usize] {
            self.inner[ply as usize][index + 1] = self.inner[(ply + 1) as usize][index]
        }
    }

    pub fn clear(&mut self, ply: isize) {
        self.len[ply as usize] = 0;
    }
}

impl Default for PVTable {
    fn default() -> Self {
        Self::new()
    }
}