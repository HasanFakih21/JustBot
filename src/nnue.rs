use crate::{
    board::Board,
    search::data::SearchData,
    types::{Piece, Side, Square},
};

const HIDDEN_SIZE: usize = 512;
const SCALE: i32 = 400;
const NUM_OUTPUT_BUCKETS: usize = 8;
const QA: i16 = 255;
const QB: i16 = 64;

pub static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../model.nnue")) };

fn output_bucket(pos: &Board) -> usize {
    let divisor = 32usize.div_ceil(NUM_OUTPUT_BUCKETS);
    ((pos.get_all_occupancy().count_bits() - 2) / divisor).min(NUM_OUTPUT_BUCKETS - 1)
}

#[inline]
fn screlu(x: i16) -> i32 {
    let y = i32::from(x).clamp(0, i32::from(QA));
    y * y
}

/// This is the quantised format that bullet outputs.
#[repr(C)]
pub struct Network {
    /// Column-Major `HIDDEN_SIZE x 768` matrix.
    /// Values have quantization of QA.
    feature_weights: [Accumulator; 768],
    /// Vector with dimension `HIDDEN_SIZE`.
    /// Values have quantization of QA.
    feature_bias: Accumulator,
    /// Column-Major `1 x (2 * HIDDEN_SIZE)`
    /// matrix, we use it like this to make the
    /// code nicer in `Network::evaluate`.
    /// Values have quantization of QB.
    output_weights: [[i16; 2 * HIDDEN_SIZE]; NUM_OUTPUT_BUCKETS],
    /// Scalar output bias.
    /// Value has quantization of QA * QB.
    output_bias: [i16; NUM_OUTPUT_BUCKETS],
}

impl Network {
    /// Calculates the output of the network, starting from the already
    /// calculated hidden layer (done efficiently during makemoves).
    pub fn evaluate(&self, data: &SearchData) -> i32 {
        // Initialise output.
        let mut output = 0;
        let stm = data.board.state.side_to_move;
        let (us, them) = match stm {
            Side::White => (&data.white_features, &data.black_features),
            Side::Black => (&data.black_features, &data.white_features),
        };

        let bucket = output_bucket(&data.board);
        let weights = &self.output_weights[bucket];

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
        output += i32::from(self.output_bias[bucket]);

        // Apply eval scale.
        output *= SCALE;

        // Remove quantisation altogether
        output /= i32::from(QA) * i32::from(QB);

        output
    }
}

/// A column of the feature-weights matrix.
/// Note the `align(64)`.
#[derive(Clone, Copy, Debug)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HIDDEN_SIZE],
}

impl Accumulator {
    /// Initialised with bias so we can just efficiently
    /// operate on it afterwards.
    pub fn new(net: &Network) -> Self {
        net.feature_bias
    }

    //64 * Piece + Square
    /// Add a feature to an accumulator.
    pub fn add_feature(&mut self, feature_idx: usize, net: &Network) {
        for (i, d) in self
            .vals
            .iter_mut()
            .zip(&net.feature_weights[feature_idx].vals)
        {
            *i += *d
        }
    }

    /// Remove a feature from an accumulator.
    pub fn remove_feature(&mut self, feature_idx: usize, net: &Network) {
        for (i, d) in self
            .vals
            .iter_mut()
            .zip(&net.feature_weights[feature_idx].vals)
        {
            *i -= *d
        }
    }

    pub fn toggle_on(&mut self, our_side: bool, piece: Piece, square: Square) {
        let feature_idx = 64 * ((!our_side as usize * 6) + piece as usize) + square as usize;
        self.add_feature(feature_idx, &NNUE);
    }

    pub fn toggle_off(&mut self, our_side: bool, piece: Piece, square: Square) {
        let feature_idx = 64 * ((!our_side as usize * 6) + piece as usize) + square as usize;
        self.remove_feature(feature_idx, &NNUE);
    }
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

        data.initialize_nnue();
        let first_eval = data.nnue_evaluate();

        println!("First Eval: {}", first_eval);
        let move_list = data.board.generate_moves(MoveGenKind::All);
        println!("{}", move_list);
        let m = data.board.parse_move("e2d1").unwrap();

        //Make the move
        data.make_move(m, 0);

        println!("Second Eval: {}", data.nnue_evaluate());

        //Unmake the move
        data.unmake_move();

        let final_eval = data.nnue_evaluate();
        println!("Final Eval: {}", final_eval);
        assert_eq!(final_eval, first_eval);
    }
}
