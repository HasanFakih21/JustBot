use crate::{
    board::Board,
    nnue::{
        HIDDEN_SIZE, MODEL, Parameters,
        cache::{AccumulatorCache, CacheData},
        input_bucket, input_context, simd,
    },
    types::{Move, Piece, ROOK_TO, Side, Square, stackvec::StackVec},
};

#[derive(Clone)]
pub struct Delta {
    pub m: Move,
    pub stm: Side,
    pub piece: Piece,
    pub captured: Option<Piece>,
}

#[derive(Clone)]
pub struct DualAccumulators {
    pub values: [Accumulator; 2],
    pub accurate: [bool; 2],
    pub delta: Option<Delta>,
}

pub type FeatureIndex = u16;

impl DualAccumulators {
    pub fn new() -> Self {
        Self {
            values: [Accumulator::new(&MODEL); 2],
            accurate: [false; 2],
            delta: None,
        }
    }

    pub fn update(
        &mut self,
        prev: &Self,
        board: &Board,
        pov: Side,
        king_square: Square,
        parameters: &Parameters,
    ) {
        let Some(delta) = &self.delta else { return };

        let from = delta.m.get_from();
        let to = delta.m.get_to();
        let stm = delta.stm;
        let moving_piece = delta.piece;
        let resulting_piece = if delta.m.is_promotion() {
            delta.m.get_promoted_piece().unwrap()
        } else {
            moving_piece
        };

        let add1 = feature_index(stm, resulting_piece, to, king_square, pov);
        let sub1 = feature_index(stm, moving_piece, from, king_square, pov);

        if let Some(captured_piece) = delta.captured {
            let capture_square = delta.m.get_capture_square();
            let sub2 = feature_index(
                stm.other(),
                captured_piece,
                capture_square,
                king_square,
                pov,
            );
            self.apply_updates(prev, [add1], [sub1, sub2], pov, parameters);
        } else if let Some(castle_kind) = delta.m.castle_direction() {
            let rook_square = board.castling_rooks[stm][castle_kind];
            let rook_landing_square = ROOK_TO[stm][castle_kind];
            let add2 = feature_index(stm, Piece::Rook, rook_landing_square, king_square, pov);
            let sub2 = feature_index(stm, Piece::Rook, rook_square, king_square, pov);
            self.apply_updates(prev, [add1, add2], [sub1, sub2], pov, parameters);
        } else {
            self.apply_updates(prev, [add1], [sub1], pov, parameters);
        }

        self.accurate[pov] = true;
    }

    pub fn apply_updates<const ADDS: usize, const SUBS: usize>(
        &mut self,
        prev: &Self,
        adds: [FeatureIndex; ADDS],
        subs: [FeatureIndex; SUBS],
        pov: Side,
        parameters: &Parameters,
    ) {
        let current = prev.values[pov as usize].vals.as_ptr();
        let updated = self.values[pov].vals.as_mut_ptr();

        unsafe {
            for i in (0..HIDDEN_SIZE).step_by(simd::I16_CHUNK) {
                let mut change = *current.add(i).cast();
                for feature_index in adds {
                    change = simd::add_i16(
                        change,
                        *parameters.feature_weights[feature_index as usize]
                            .vals
                            .as_ptr()
                            .add(i)
                            .cast(),
                    );
                }

                for feature_index in subs {
                    change = simd::sub_i16(
                        change,
                        *parameters.feature_weights[feature_index as usize]
                            .vals
                            .as_ptr()
                            .add(i)
                            .cast(),
                    );
                }

                *updated.add(i).cast() = change;
            }
        }
    }

    pub fn refresh(
        &mut self,
        board: &Board,
        pov: Side,
        parameters: &Parameters,
        cache: &mut AccumulatorCache,
    ) {
        let king_square = board.get_king_square(pov);
        let (input_bucket, hm) = input_context(king_square ^ (56 * pov as u8));
        let cache_data = cache.get_mut(pov, hm, input_bucket);

        let mut adds = StackVec::<FeatureIndex, 64>::new();
        let mut subs = StackVec::<FeatureIndex, 64>::new();

        for side in Side::ALL {
            for piece in Piece::ALL {
                let piece_bb = board.get_piece_bb(side, piece);
                let to_add = piece_bb & !(cache_data.pieces[piece] & cache_data.occupancies[side]);
                let to_sub = !piece_bb & (cache_data.pieces[piece] & cache_data.occupancies[side]);

                for square in to_add.iter() {
                    adds.push(feature_index(side, piece, square, king_square, pov));
                }

                for square in to_sub.iter() {
                    subs.push(feature_index(side, piece, square, king_square, pov));
                }
            }
        }

        // Apply updates
        update_from_cache(adds, subs, parameters, cache_data);

        cache_data.pieces = board.state.pieces;
        cache_data.occupancies = board.state.occupancies;

        self.values[pov] = cache_data.accumulator;
        self.accurate[pov] = true;
    }
}

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
const REGISTERS: usize = 8;
#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
const UNROLL: usize = simd::I16_CHUNK * REGISTERS;

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
pub fn update_from_cache(
    adds: StackVec<FeatureIndex, 64>,
    subs: StackVec<FeatureIndex, 64>,
    parameters: &Parameters,
    cache_data: &mut CacheData,
) {
    unsafe {
        let mut registers = [simd::zeroed(); REGISTERS];

        for i in (0..HIDDEN_SIZE).step_by(UNROLL) {
            let src = cache_data.accumulator.vals.as_mut_ptr().add(i);
            for (r_idx, r) in registers.iter_mut().enumerate() {
                *r = *src.add(r_idx * simd::I16_CHUNK).cast();
            }

            for &add in adds.iter() {
                let weights = parameters.feature_weights[add as usize]
                    .vals
                    .as_ptr()
                    .add(i);

                for (r_idx, r) in registers.iter_mut().enumerate() {
                    *r = simd::add_i16(*r, *weights.add(r_idx * simd::I16_CHUNK).cast());
                }
            }

            for &sub in subs.iter() {
                let weights = parameters.feature_weights[sub as usize]
                    .vals
                    .as_ptr()
                    .add(i);
                for (r_idx, r) in registers.iter_mut().enumerate() {
                    *r = simd::sub_i16(*r, *weights.add(r_idx * simd::I16_CHUNK).cast());
                }
            }

            for (r_idx, r) in registers.into_iter().enumerate() {
                *src.add(r_idx * simd::I16_CHUNK).cast() = r;
            }
        }
    }
}

#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
pub fn update_from_cache(
    adds: StackVec<FeatureIndex, 64>,
    subs: StackVec<FeatureIndex, 64>,
    parameters: &Parameters,
    cache_data: &mut CacheData,
) {
    let acc = &mut cache_data.accumulator.vals;
    for &feature in adds.iter() {
        let weights = &parameters.feature_weights[feature as usize].vals;
        for (output, &weight) in acc.iter_mut().zip(weights) {
            *output += weight;
        }
    }

    for &feature in subs.iter() {
        let weights = &parameters.feature_weights[feature as usize].vals;
        for (output, &weight) in acc.iter_mut().zip(weights) {
            *output -= weight;
        }
    }
}

impl Default for DualAccumulators {
    fn default() -> Self {
        Self::new()
    }
}

/// A column of the feature-weights matrix.
/// Note the `align(64)`.
#[derive(Clone, Copy, Debug)]
#[repr(C, align(64))]
pub struct Accumulator {
    pub vals: [i16; HIDDEN_SIZE],
}

impl Accumulator {
    /// Initialised with bias so we can just efficiently
    /// operate on it afterwards.
    pub fn new(net: &Parameters) -> Self {
        net.feature_bias
    }
}

#[inline]
pub fn feature_index(
    side: Side,
    piece: Piece,
    square: Square,
    king_square: Square,
    pov: Side,
) -> FeatureIndex {
    let square = square ^ (56 * pov as u8);
    let king_square = king_square ^ (56 * pov as u8);

    input_bucket(king_square) as FeatureIndex * 768
        + ((((pov != side) as FeatureIndex * 6) + piece as FeatureIndex) * 64)
        + (square as FeatureIndex ^ ((king_square.to_file() > 3) as FeatureIndex * 7))
}
