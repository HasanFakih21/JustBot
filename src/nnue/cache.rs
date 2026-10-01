use crate::{
    nnue::{Aligned, INPUT_BUCKETS, L1, Parameters},
    types::{BitBoard, Side},
};

// Finny Tables
// [POV][Horizontally Mirrored][Input Bucket]
#[derive(Clone)]
pub struct AccumulatorCache(Box<[[[CacheData; INPUT_BUCKETS]; 2]; 2]>);

impl AccumulatorCache {
    pub fn new(parameters: &Parameters) -> Self {
        Self(Box::new([[[CacheData::new(parameters); INPUT_BUCKETS]; 2]; 2]))
    }

    pub fn get_mut(&mut self, pov: Side, hm: bool, input_bucket: usize) -> &mut CacheData {
        &mut self.0[pov][hm as usize][input_bucket]
    }
}

#[derive(Clone, Copy)]
pub struct CacheData {
    pub values: Aligned<[i16; L1]>,
    pub pieces: [BitBoard; 6],
    pub occupancies: [BitBoard; 2],
}

impl CacheData {
    pub fn new(parameters: &Parameters) -> Self {
        Self {
            values: parameters.feature_bias,
            pieces: [BitBoard(0); 6],
            occupancies: [BitBoard(0); 2],
        }
    }
}
