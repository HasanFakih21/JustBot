use crate::{
    board::Board,
    nnue::{
        HIDDEN_SIZE, MODEL, Parameters,
        cache::{AccumulatorCache, CacheData},
        input_bucket, input_context,
    },
    types::{CASTLING_ROOK_SQAURES, Move, Piece, Side, Square, stackvec::StackVec},
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

    pub fn update(&mut self, prev: &Self, pov: Side, king_square: Square, parameters: &Parameters) {
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
        } else if let Some(castle_kind) = delta.m.castle_kind() {
            let offset = [1, -1];
            let rook_square = CASTLING_ROOK_SQAURES[stm][castle_kind];
            let rook_landing_square = from.shift(offset[castle_kind]).unwrap();
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
            for i in 0..HIDDEN_SIZE {
                let mut change = *current.add(i);
                for feature_index in adds {
                    change += parameters.feature_weights[feature_index as usize].vals[i];
                }

                for feature_index in subs {
                    change -= parameters.feature_weights[feature_index as usize].vals[i];
                }

                *updated.add(i) = change;
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

pub fn update_from_cache(
    adds: StackVec<FeatureIndex, 64>,
    subs: StackVec<FeatureIndex, 64>,
    parameters: &Parameters,
    cache_data: &mut CacheData,
) {
    let acc = cache_data.accumulator.vals.as_mut_ptr();

    unsafe {
        for i in 0..HIDDEN_SIZE {
            let mut change = *acc.add(i);
            for feature_index in adds.iter() {
                change += parameters.feature_weights[*feature_index as usize].vals[i];
            }

            for feature_index in subs.iter() {
                change -= parameters.feature_weights[*feature_index as usize].vals[i];
            }

            *acc.add(i) = change;
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
