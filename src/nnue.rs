use crate::{
    board::Board,
    types::{
        KING_SIDE_ROOK_BLACK, KING_SIDE_ROOK_WHITE, MAX_PLY, Move, MoveKind, Piece,
        QUEEN_SIDE_ROOK_BLACK, QUEEN_SIDE_ROOK_WHITE, Side, Square, to_file_bb,
    },
};

const HIDDEN_SIZE: usize = 512;
const SCALE: i32 = 400;
const NUM_OUTPUT_BUCKETS: usize = 8;
const QA: i16 = 255;
const QB: i16 = 64;

pub static MODEL: Parameters = unsafe { std::mem::transmute(*include_bytes!("../model.nnue")) };

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

    //Pushes the current accumulators into the stack
    pub fn push(&mut self) {
        self.index += 1;
        self.stack[self.index] = self.stack[self.index - 1].clone();
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
        let kind = m.get_kind();
        let from = m.get_from();
        let to = m.get_to();
        let stm = board.state.side_to_move;
        let moving_piece = board.get_piece_at_square(from).unwrap().1;

        //Need to toggle off extra captured piece in case of capture
        if kind.is_capture() {
            let capture_square = m.get_capture_square();
            let (_, captured_piece) = board.get_piece_at_square(capture_square).unwrap();

            self.toggle_accumulators_off(board, stm.other(), captured_piece, capture_square);
        }

        //Need to toggle rook in case of castling
        if kind == MoveKind::KingCastle {
            debug_assert!(!(from.to_bb() & to_file_bb(Square::E4)).is_empty());
            let king_rook_square = match stm {
                Side::White => KING_SIDE_ROOK_WHITE,
                Side::Black => KING_SIDE_ROOK_BLACK,
            };

            self.toggle_accumulators_off(board, stm, Piece::Rook, king_rook_square);
            self.toggle_accumulators_on(board, stm, Piece::Rook, from.shift(1).unwrap());
        }

        //Need to toggle rook in case of castling
        if kind == MoveKind::QueenCastle {
            debug_assert!(!(from.to_bb() & to_file_bb(Square::E4)).is_empty());
            let queen_rook_square = match stm {
                Side::White => QUEEN_SIDE_ROOK_WHITE,
                Side::Black => QUEEN_SIDE_ROOK_BLACK,
            };

            self.toggle_accumulators_off(board, stm, Piece::Rook, queen_rook_square);
            self.toggle_accumulators_on(board, stm, Piece::Rook, from.shift(-1).unwrap());
        }

        //Need to handle promotions
        if kind.is_promotion() {
            let promotion_piece = m.get_promoted_piece().unwrap();
            self.toggle_accumulators_off(board, stm, moving_piece, from);
            self.toggle_accumulators_on(board, stm, promotion_piece, to);
        } else {
            self.toggle_accumulators_off(board, stm, moving_piece, from);
            self.toggle_accumulators_on(board, stm, moving_piece, to);
        }
    }

    pub fn toggle_accumulators_on(
        &mut self,
        board: &Board,
        pov: Side,
        piece: Piece,
        square: Square,
    ) {
        let white_king = board.get_king_square(Side::White);
        let black_king = board.get_king_square(Side::Black) ^ 56;

        self.stack[self.index].values[Side::White as usize].toggle_on(
            pov == Side::White,
            piece,
            square,
            white_king,
        );
        self.stack[self.index].values[Side::Black as usize].toggle_on(
            pov == Side::Black,
            piece,
            square ^ 56,
            black_king,
        );
    }

    pub fn toggle_accumulators_off(
        &mut self,
        board: &Board,
        pov: Side,
        piece: Piece,
        square: Square,
    ) {
        let white_king = board.get_king_square(Side::White);
        let black_king = board.get_king_square(Side::Black) ^ 56;

        self.stack[self.index].values[Side::White as usize].toggle_off(
            pov == Side::White,
            piece,
            square,
            white_king,
        );
        self.stack[self.index].values[Side::Black as usize].toggle_off(
            pov == Side::Black,
            piece,
            square ^ 56,
            black_king,
        );
    }

    pub fn clear_features(&mut self) {
        self.stack[self.index].values = [Accumulator::new(&MODEL); 2];
    }

    pub fn full_refresh(&mut self, board: &Board) {
        for square in board.get_all_occupancy().iter() {
            if let Some((side, piece)) = board.get_piece_at_square(square) {
                self.toggle_accumulators_on(board, side, piece, square);
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
    feature_weights: [Accumulator; 768],
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

    pub fn toggle_on(&mut self, our_side: bool, piece: Piece, square: Square, king_square: Square) {
        self.add_feature(feature_index(our_side, piece, square, king_square), &MODEL);
    }

    pub fn toggle_off(
        &mut self,
        our_side: bool,
        piece: Piece,
        square: Square,
        king_square: Square,
    ) {
        self.remove_feature(feature_index(our_side, piece, square, king_square), &MODEL);
    }
}

#[inline]
pub fn feature_index(our_side: bool, piece: Piece, square: Square, king_square: Square) -> usize {
    let king_file = king_square.to_file();

    (((!our_side as usize * 6) + piece as usize) * 64)
        + (square as usize ^ ((king_file > 3) as usize * 7))
}

#[inline]
pub fn input_context(king_square: Square) -> bool {
    //Which Half of the board the king is on
    king_square.to_file() > 3
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

        //Make the move
        data.make_move(m, 0);

        println!("Second Eval: {}", data.network.evaluate(&data.board));

        //Unmake the move
        data.unmake_move();

        let final_eval = data.network.evaluate(&data.board);
        println!("Final Eval: {}", final_eval);
        assert_eq!(final_eval, first_eval);
    }
}
