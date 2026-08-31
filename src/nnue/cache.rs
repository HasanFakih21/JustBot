use crate::{
    nnue::{NUM_INPUT_BUCKETS, Parameters, accumulator::Accumulator},
    types::{BitBoard, Side},
};

// Finny Tables
// [POV][Horizontally Mirrored][Input Bucket]
#[derive(Clone)]
pub struct AccumulatorCache(Box<[[[CacheData; NUM_INPUT_BUCKETS]; 2]; 2]>);

impl AccumulatorCache {
    pub fn new(parameters: &Parameters) -> Self {
        Self(Box::new([[[CacheData::new(parameters); NUM_INPUT_BUCKETS]; 2]; 2]))
    }

    pub fn get_mut(&mut self, pov: Side, hm: bool, input_bucket: usize) -> &mut CacheData {
        &mut self.0[pov][hm as usize][input_bucket]
    }
}

#[derive(Clone, Copy)]
pub struct CacheData {
    pub accumulator: Accumulator,
    pub pieces: [BitBoard; 6],
    pub occupancies: [BitBoard; 2],
}

impl CacheData {
    pub fn new(parameters: &Parameters) -> Self {
        Self {
            accumulator: Accumulator::new(parameters),
            pieces: [BitBoard(0); 6],
            occupancies: [BitBoard(0); 2],
        }
    }
}
