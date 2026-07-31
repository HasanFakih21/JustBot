use crate::{
    board::Board,
    nnue::{HIDDEN_SIZE, MODEL, Parameters, input_bucket},
    types::{CASTLING_ROOK_SQAURES, Move, Piece, Side, Square},
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
                    change += parameters.feature_weights[feature_index].vals[i];
                }

                for feature_index in subs {
                    change -= parameters.feature_weights[feature_index].vals[i];
                }

                *updated.add(i) = change;
            }
        }
    }

    pub fn refresh(&mut self, board: &Board, pov: Side, parameters: &Parameters) {
        self.values[pov] = Accumulator::new(parameters);

        for square in board.get_all_occupancy().iter() {
            if let Some((side, piece)) = board.get_piece_at_square(square) {
                self.values[pov as usize].add_feature(
                    feature_index(side, piece, square, board.get_king_square(pov), pov),
                    parameters,
                );
            }
        }

        self.accurate[pov] = true;
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

    /// Add a feature to an accumulator.
    pub fn add_feature(&mut self, feature_idx: usize, net: &Parameters) {
        for (i, d) in self
            .vals
            .iter_mut()
            .zip(&net.feature_weights[feature_idx].vals)
        {
            *i += *d
        }
    }

    // /// Remove a feature from an accumulator.
    // pub fn remove_feature(&mut self, feature_idx: usize, net: &Parameters) {
    //     for (i, d) in self
    //         .vals
    //         .iter_mut()
    //         .zip(&net.feature_weights[feature_idx].vals)
    //     {
    //         *i -= *d
    //     }
    // }
}

pub type FeatureIndex = usize;

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

    input_bucket(king_square) * 768
        + ((((pov != side) as usize * 6) + piece as usize) * 64)
        + (square as usize ^ ((king_square.to_file() > 3) as usize * 7))
}
