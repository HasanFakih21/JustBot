use crate::{
    board::Board,
    nnue::accumulator::{Accumulator, Delta, DualAccumulators},
    types::{MAX_PLY, Move, Piece, Side, Square},
};

mod accumulator;

const HIDDEN_SIZE: usize = 512;
const SCALE: i32 = 400;
const NUM_OUTPUT_BUCKETS: usize = 8;
const QA: i16 = 255;
const QB: i16 = 64;

#[rustfmt::skip]
const BUCKET_LAYOUT: [usize; 32] = [
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    2, 2, 2, 2,
    1, 1, 0, 0
];

const NUM_INPUT_BUCKETS: usize = 4;

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

    pub fn can_update(&self, pov: Side) -> Option<usize> {
        for i in (0..=self.index).rev() {
            if self.stack[i].accurate[pov] {
                return Some(i);
            }

            let Some(delta) = &self.stack[i].delta else {
                return None;
            };

            let needs_refresh = delta.piece == Piece::King
                && delta.stm == pov
                && input_context(delta.m.get_from() ^ (56 * (delta.stm == Side::Black) as u8))
                    != input_context(delta.m.get_to() ^ (56 * (delta.stm == Side::Black) as u8));

            if needs_refresh {
                return None;
            }
        }

        None
    }

    pub fn push(&mut self, board: &Board, m: Move) {
        self.index += 1;
        self.stack[self.index].delta = Some(Delta {
            m,
            stm: board.state.side_to_move,
            piece: board.get_piece_at_square(m.get_from()).unwrap().1,
            captured: if m.is_capture() {
                Some(board.get_piece_at_square(m.get_capture_square()).unwrap().1)
            } else {
                None
            },
        });
        self.stack[self.index].accurate = [false; 2];
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn evaluate(&mut self, board: &Board) -> i32 {
        for pov in [Side::White, Side::Black] {
            if self.stack[self.index].accurate[pov] {
                continue;
            }

            match self.can_update(pov) {
                Some(last_accurate) => {
                    // Update all the not yet updated accumulators
                    let king_square = board.get_king_square(pov);
                    for index in last_accurate..self.index {
                        if let Some((prev, [current, ..])) =
                            self.stack.split_at_mut_checked(index + 1)
                        {
                            current.update(&prev[index], pov, king_square, self.parameters);
                        }
                    }
                }
                None => self.stack[self.index].refresh(board, pov, self.parameters),
            }
        }

        self.output_layer(board)
    }

    pub fn output_layer(&self, board: &Board) -> i32 {
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

        output /= i32::from(QA);
        output += i32::from(self.parameters.output_bias[bucket]);
        output *= SCALE;
        output /= i32::from(QA) * i32::from(QB);

        output
    }

    pub fn full_refresh(&mut self, board: &Board) {
        for pov in [Side::White, Side::Black] {
            self.stack[self.index].refresh(board, pov, self.parameters);
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
pub struct Parameters {
    feature_weights: [Accumulator; 768 * NUM_INPUT_BUCKETS],
    feature_bias: Accumulator,
    output_weights: [[i16; 2 * HIDDEN_SIZE]; NUM_OUTPUT_BUCKETS],
    output_bias: [i16; NUM_OUTPUT_BUCKETS],
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
