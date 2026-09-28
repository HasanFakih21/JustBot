use super::{scalar, vectorized};
use crate::{
    board::Board,
    nnue::{L1, MODEL, Network, OUTPUT_BUCKETS, accumulator::Accumulator},
    types::{STARTING_FEN, Side},
};

#[test]
fn vectorized_matches_scalar_at_clipping_boundaries() {
    let mut us = Accumulator { vals: [0; L1] };
    let mut them = us;
    let values = [i16::MIN, -256, -1, 0, 1, 22, 23, 127, 128, 254, 255, 256, i16::MAX];
    for offset in 0..values.len() {
        for i in 0..L1 {
            us.vals[i] = values[(i + offset) % values.len()];
            them.vals[i] = values[(i * 7 + offset) % values.len()];
        }
        for bucket in 0..OUTPUT_BUCKETS {
            assert_eq!(
                scalar::forward(&us, &them, &MODEL, bucket),
                vectorized::forward(&us, &them, &MODEL, bucket),
                "offset {offset}, bucket {bucket}"
            );
        }
    }
}

#[test]
fn vectorized_matches_scalar_on_random_accumulators() {
    let mut seed = 0x1234_5678_u32;
    let mut us = Accumulator { vals: [0; L1] };
    let mut them = us;
    for sample in 0..64 {
        for v in us.vals.iter_mut().chain(them.vals.iter_mut()) {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            // Include sparse, partially active, and saturated pairwise features.
            *v = if sample % 2 == 0 { ((seed >> 16) % 768) as i16 - 256 } else { (seed >> 16) as i16 };
        }
        for bucket in 0..OUTPUT_BUCKETS {
            assert_eq!(
                scalar::forward(&us, &them, &MODEL, bucket),
                vectorized::forward(&us, &them, &MODEL, bucket),
                "sample {sample}, bucket {bucket}"
            );
        }
    }
}

#[test]
fn vectorized_matches_scalar_on_positions() {
    let mut net = Network::new();
    for fen in [
        STARTING_FEN,
        "rnbq1rk1/pp3p2/4pnpp/1p1p2N1/3P4/1P2P3/PBPbKPPP/R6R w - - 2 4",
        "R2r4/pK1b4/1n4NB/7P/8/3Q4/6k1/4q3 b - - 0 1",
        "8/8/8/3k4/8/4K3/8/8 w - - 0 1",
    ] {
        let board = Board::from_fen(fen).unwrap();
        net.full_refresh(&board);
        let accs = &net.stack[net.index].values;
        for stm in [Side::White, Side::Black] {
            for bucket in 0..OUTPUT_BUCKETS {
                assert_eq!(
                    scalar::forward(&accs[stm], &accs[!stm], &MODEL, bucket),
                    vectorized::forward(&accs[stm], &accs[!stm], &MODEL, bucket),
                    "FEN {fen}, side {stm:?}, bucket {bucket}"
                );
            }
        }
    }
}
