use crate::{
    board::Board,
    types::{CASTLING_ROOK_SQAURES, MAX_PLY, Move, Piece, Side, Square},
};

const HIDDEN_SIZE: usize = 512;
const SCALE: i32 = 400;
const NUM_OUTPUT_BUCKETS: usize = 8;
const QA: i16 = 255;
const QB: i16 = 64;

#[rustfmt::skip]
const BUCKET_LAYOUT: [usize; 32] = [
    0, 0, 0, 0, 
    0, 0, 0, 0,
    1, 1, 1, 1, 
    1, 1, 1, 1,
    1, 1, 1, 1,
    2, 2, 2, 2, 
    2, 2, 2, 2,
    2, 2, 2, 2,
];

const NUM_INPUT_BUCKETS: usize = 3;

pub static MODEL: Parameters = unsafe { std::mem::transmute(*include_bytes!("../model.nnue")) };

pub type FeatureIndex = usize;

pub struct Network {
    parameters: &'static Parameters,
    stack: Box<[DualAccumulators]>,
    index: usize,
}

impl Network {
    pub fn new() -> Self {
        Network {
            parameters: &MODEL,
            stack: vec![DualAccumulators::new(); MAX_PLY].into_boxed_slice(),
            index: 0,
        }
    }

    pub fn push(&mut self) {
        self.index += 1;
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn evaluate(&self, board: &Board) -> i32 {
        // Initialise output.
        let mut output = 0;
        let stm = board.state.side_to_move;
        let (us, them) = (
            self.stack[self.index].values[stm as usize],
            self.stack[self.index].values[stm.other() as usize],
        );

        let bucket = output_bucket(board);
        let weights = &self.parameters.output_weights[bucket];

        // Side-To-Move Accumulator -> Output.
        for (&input, &weight) in us.vals.iter().zip(&weights[..HIDDEN_SIZE]) {
            output += screlu(input) * i32::from(weight);
        }

        // Not-Side-To-Move Accumulator -> Output.
        for (&input, &weight) in them.vals.iter().zip(&weights[HIDDEN_SIZE..]) {
            output += screlu(input) * i32::from(weight);
        }

        // Reduce quantization from QA * QA * QB to QA * QB.
        output /= i32::from(QA);

        // Add bias.
        output += i32::from(self.parameters.output_bias[bucket]);

        // Apply eval scale.
        output *= SCALE;

        // Remove quantisation altogether
        output /= i32::from(QA) * i32::from(QB);

        output
    }

    pub fn update(&mut self, board: &Board, m: Move) {
        let from = m.get_from();
        let to = m.get_to();
        let stm = board.state.side_to_move;
        let moving_piece = board.get_piece_at_square(from).unwrap().1;
        let resulting_piece = if m.is_promotion() {
            m.get_promoted_piece().unwrap()
        } else {
            moving_piece
        };

        for pov in [Side::White, Side::Black] {
            let king_square = board.get_king_square(pov);
            let add1 = feature_index(stm, resulting_piece, to, king_square, pov);
            let sub1 = feature_index(stm, moving_piece, from, king_square, pov);

            if m.is_capture() {
                let capture_square = m.get_capture_square();
                let (_, captured_piece) = board.get_piece_at_square(capture_square).unwrap();
                let sub2 = feature_index(
                    stm.other(),
                    captured_piece,
                    capture_square,
                    king_square,
                    pov,
                );
                self.apply_updates([add1], [sub1, sub2], pov);
            } else if let Some(castle_kind) = m.castle_kind() {
                let offset = [1, -1];
                let rook_square = CASTLING_ROOK_SQAURES[stm][castle_kind];
                let rook_landing_square = from.shift(offset[castle_kind]).unwrap();
                let add2 = feature_index(stm, Piece::Rook, rook_landing_square, king_square, pov);
                let sub2 = feature_index(stm, Piece::Rook, rook_square, king_square, pov);
                self.apply_updates([add1, add2], [sub1, sub2], pov);
            } else {
                self.apply_updates([add1], [sub1], pov);
            }
        }
    }

    pub fn apply_updates<const ADDS: usize, const SUBS: usize>(
        &mut self,
        adds: [FeatureIndex; ADDS],
        subs: [FeatureIndex; SUBS],
        pov: Side,
    ) {
        let current = self.stack[self.index - 1].values[pov as usize]
            .vals
            .as_ptr();
        let updated = self.stack[self.index].values[pov].vals.as_mut_ptr();

        unsafe {
            for i in 0..HIDDEN_SIZE {
                let mut change = *current.add(i);
                for feature_index in adds {
                    change += self.parameters.feature_weights[feature_index].vals[i];
                }

                for feature_index in subs {
                    change -= self.parameters.feature_weights[feature_index].vals[i];
                }

                *updated.add(i) = change;
            }
        }
    }

    pub fn clear_features(&mut self) {
        self.stack[self.index].values = [Accumulator::new(&MODEL); 2];
    }

    pub fn full_refresh(&mut self, board: &Board) {
        for square in board.get_all_occupancy().iter() {
            if let Some((side, piece)) = board.get_piece_at_square(square) {
                for pov in [Side::White, Side::Black] {
                    self.stack[self.index].values[pov as usize].add_feature(
                        feature_index(side, piece, square, board.get_king_square(pov), pov),
                        self.parameters,
                    );
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct DualAccumulators {
    pub values: [Accumulator; 2],
}

impl DualAccumulators {
    pub fn new() -> Self {
        Self {
            values: [Accumulator::new(&MODEL); 2],
        }
    }
}

impl Default for DualAccumulators {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

/// This is the quantised format that bullet outputs.
#[repr(C)]
pub struct Parameters {
    feature_weights: [Accumulator; 768 * NUM_INPUT_BUCKETS],
    feature_bias: Accumulator,
    output_weights: [[i16; 2 * HIDDEN_SIZE]; NUM_OUTPUT_BUCKETS],
    output_bias: [i16; NUM_OUTPUT_BUCKETS],
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

    /// Remove a feature from an accumulator.
    pub fn remove_feature(&mut self, feature_idx: usize, net: &Parameters) {
        for (i, d) in self
            .vals
            .iter_mut()
            .zip(&net.feature_weights[feature_idx].vals)
        {
            *i -= *d
        }
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

    input_bucket(king_square) * 768
        + ((((pov != side) as usize * 6) + piece as usize) * 64)
        + (square as usize ^ ((king_square.to_file() > 3) as usize * 7))
}

#[inline]
// Input Bucket, Which Half
pub fn input_context(king_square: Square) -> (usize, bool) {
    (input_bucket(king_square), king_square.to_file() > 3)
}

#[inline]
fn input_bucket(king_square: Square) -> usize {
    let (rank, file) = king_square.to_rank_and_file();
    BUCKET_LAYOUT[rank * 4 + (file.min(7 - file))]
}

#[inline]
fn output_bucket(pos: &Board) -> usize {
    let divisor = 32usize.div_ceil(NUM_OUTPUT_BUCKETS);
    ((pos.get_all_occupancy().count_bits() - 2) / divisor).min(NUM_OUTPUT_BUCKETS - 1)
}

#[inline]
fn screlu(x: i16) -> i32 {
    let y = i32::from(x).clamp(0, i32::from(QA));
    y * y
}

#[cfg(test)]
mod tests {

    use crate::{
        board::{Board, movegen::MoveGenKind},
        nnue::output_bucket,
        search::data::SearchData,
        types::STARTING_FEN,
    };

    #[test]
    fn test_output_bucket() {
        let data = SearchData {
            board: Board::from_fen(STARTING_FEN).unwrap(),
            ..Default::default()
        };

        let bucket = output_bucket(&data.board);
        assert_eq!(bucket, 7);
    }

    #[test]
    fn test_nnue_make_unmake() {
        let mut data = SearchData {
            board: Board::from_fen("rnbq1rk1/pp3p2/4pnpp/1p1p2N1/3P4/1P2P3/PBPbKPPP/R6R w - - 2 4")
                .unwrap(),
            ..Default::default()
        };

        data.network.full_refresh(&data.board);
        let first_eval = data.network.evaluate(&data.board);

        println!("First Eval: {}", first_eval);
        let move_list = data.board.generate_moves(MoveGenKind::All);
        println!("{}", move_list);
        let m = data.board.parse_move("e2d1").unwrap();

        // Make the move
        data.make_move(m, 0);

        println!("Second Eval: {}", data.network.evaluate(&data.board));

        // Unmake the move
        data.unmake_move();

        let final_eval = data.network.evaluate(&data.board);
        println!("Final Eval: {}", final_eval);
        assert_eq!(final_eval, first_eval);
    }
}
