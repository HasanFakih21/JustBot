use crate::{
    board::Board,
    nnue::{
        accumulator::{Accumulator, Delta},
        cache::AccumulatorCache,
    },
    types::{MAX_PLY, Move, OptionPiece, Piece, Side, Square},
};

mod accumulator;
mod cache;
mod forward {
    #[cfg(target_feature = "avx2")]
    mod vectorized;
    #[cfg(target_feature = "avx2")]
    pub use vectorized::*;

    #[cfg(not(target_feature = "avx2"))]
    mod scalar;
    #[cfg(not(target_feature = "avx2"))]
    pub use scalar::*;
}

mod simd {
    #[cfg(target_feature = "avx512f")]
    mod avx512;
    #[cfg(target_feature = "avx512f")]
    pub use avx512::*;

    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    mod avx2;
    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    pub use avx2::*;

    #[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
    mod scalar;
    #[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
    pub use scalar::*;
}

const SCALE: i32 = 400;

const L1: usize = 1024;
const L2: usize = 16;
const L3: usize = 32;

const Q0: i16 = 255;
const Q: i16 = 64;

const INPUT_BUCKETS: usize = 8;
const OUTPUT_BUCKETS: usize = 8;

#[rustfmt::skip]
const BUCKET_LAYOUT: [usize; 32] = [
    0,  1,  2,  3,
    4,  4,  5,  5,
    6,  6,  6,  6,
    6,  6,  6,  6,
    7,  7,  7,  7,
    7,  7,  7,  7,
    7,  7,  7,  7,
    7,  7,  7,  7,
];

pub static MODEL: Parameters = unsafe { std::mem::transmute(*include_bytes!(env!("MODEL"))) };

pub struct Network {
    parameters: &'static Parameters,
    stack: Box<[Accumulator]>,
    index: usize,
    cache: AccumulatorCache,
}

impl Network {
    pub fn new() -> Self {
        Network {
            parameters: &MODEL,
            stack: vec![Accumulator::new(); MAX_PLY].into_boxed_slice(),
            index: 0,
            cache: AccumulatorCache::new(&MODEL),
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
                && input_context(delta.m.from() ^ (56 * (delta.stm == Side::Black) as u8))
                    != input_context(delta.m.to() ^ (56 * (delta.stm == Side::Black) as u8));

            if needs_refresh {
                return None;
            }
        }

        None
    }

    pub fn push(&mut self, board: &Board, m: Move) {
        debug_assert!(board.piece_at_square(m.from()) != OptionPiece::None);
        self.index += 1;
        self.stack[self.index].delta = Some(Delta {
            m,
            stm: board.state.side_to_move,
            piece: board.piece_at_square(m.from()).unwrap().kind(),
            captured: if m.is_capture() {
                Some(board.piece_at_square(m.capture_square()).unwrap().kind())
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
                    let king_square = board.king_square(pov);
                    for index in last_accurate..self.index {
                        if let Some((prev, [current, ..])) = self.stack.split_at_mut_checked(index + 1) {
                            current.update(&prev[index], board, pov, king_square, self.parameters);
                        }
                    }
                }
                None => self.stack[self.index].refresh(board, pov, self.parameters, &mut self.cache),
            }
        }

        let eval = self.output_transform(board);
        #[cfg(not(feature = "datagen"))]
        let eval = board.scale_eval(eval);
        eval
    }

    pub fn output_transform(&self, board: &Board) -> i32 {
        let bucket = output_bucket(board);
        let parameters = self.parameters;

        let ft_out = forward::activate_features(&self.stack[self.index], board.state.side_to_move);
        let l1_out = forward::propogate_l1(&ft_out, bucket, parameters);
        let l2_out = forward::propogate_l2(&l1_out, bucket, parameters);

        forward::propogate_l3(&l2_out, bucket, parameters)
    }

    pub fn full_refresh(&mut self, board: &Board) {
        for pov in [Side::White, Side::Black] {
            self.stack[self.index].refresh(board, pov, self.parameters, &mut self.cache);
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
    feature_weights: Aligned<[[i16; L1]; 768 * INPUT_BUCKETS]>,
    feature_bias: Aligned<[i16; L1]>,
    l1_weights: Aligned<[[i8; L2 * L1]; OUTPUT_BUCKETS]>,
    l1_bias: Aligned<[[i32; L2]; OUTPUT_BUCKETS]>,
    l2_weights: Aligned<[[i32; L3 * L2]; OUTPUT_BUCKETS]>,
    l2_bias: Aligned<[[i32; L3]; OUTPUT_BUCKETS]>,
    l3_weights: Aligned<[[i32; L3]; OUTPUT_BUCKETS]>,
    l3_bias: Aligned<[i32; OUTPUT_BUCKETS]>,
}

#[repr(align(64))]
#[derive(Clone, Copy, Debug)]
struct Aligned<T> {
    data: T,
}

impl<T> Aligned<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> std::ops::Deref for Aligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> std::ops::DerefMut for Aligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
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
    let divisor = 32usize.div_ceil(OUTPUT_BUCKETS);
    ((pos.all_occupancy().count_bits() - 2) / divisor).min(OUTPUT_BUCKETS - 1)
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
            board: Board::from_fen("rnbq1rk1/pp3p2/4pnpp/1p1p2N1/3P4/1P2P3/PBPbKPPP/R6R w - - 2 4").unwrap(),
            ..Default::default()
        };

        data.network.full_refresh(&data.board);
        let first_eval = data.network.evaluate(&data.board);

        println!("First Eval: {}", first_eval);
        let _ = data.board.generate_moves(MoveGenKind::All);
        let m = data.board.parse_move("e2d1").unwrap();

        // Make the move
        data.make_move(m, 0, 1);

        println!("Second Eval: {}", data.network.evaluate(&data.board));

        // Unmake the move
        data.unmake_move();

        let final_eval = data.network.evaluate(&data.board);
        println!("Final Eval: {}", final_eval);
        assert_eq!(final_eval, first_eval);
    }
}
