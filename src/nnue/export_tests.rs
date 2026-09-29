//! Reference the on-disk Bullet layout directly, independently of Parameters and
//! its compile-time reordering. Keep f64 activations to check the f32 backends.
use super::*;

fn exported_forward(us: &Accumulator, them: &Accumulator, bucket: usize) -> f64 {
    let bytes = include_bytes!(env!("MODEL"));
    let first_weights = (768 * INPUT_BUCKETS + 1) * L1 * 2;
    let first_bias = first_weights + OUTPUT_BUCKETS * L2 * L1;
    let second_weights = first_bias + OUTPUT_BUCKETS * L2 * 4;
    let second_bias = second_weights + OUTPUT_BUCKETS * L2 * L3 * 4;
    let output_weights = second_bias + OUTPUT_BUCKETS * L3 * 4;
    let output_bias = output_weights + OUTPUT_BUCKETS * L3 * 4;
    // Bullet pads the complete export to 64 bytes.
    assert_eq!(bytes.len(), (output_bias + OUTPUT_BUCKETS * 4).next_multiple_of(64));
    let read = |offset: usize| i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as f64;

    let mut hidden = [0.0; L2];
    for (j, value) in hidden.iter_mut().enumerate() {
        let mut sum = read(first_bias + (bucket * L2 + j) * 4);
        for (pov, acc) in [us, them].into_iter().enumerate() {
            for i in 0..L1 / 2 {
                let a = i32::from(acc.vals[i]).clamp(0, 255);
                let b = i32::from(acc.vals[i + L1 / 2]).clamp(0, 255);
                let weight = bytes[first_weights + (bucket * L2 + j) * L1 + pov * L1 / 2 + i] as i8;
                sum += f64::from((a * b) / 256) * f64::from(weight);
            }
        }
        *value = (sum / 32768.0).clamp(0.0, 1.0).powi(2);
    }
    let mut output = read(output_bias + bucket * 4) / 4096.0;
    for j in 0..L3 {
        let mut sum = read(second_bias + (bucket * L3 + j) * 4) / 4096.0;
        for (i, &value) in hidden.iter().enumerate() {
            sum += value * read(second_weights + ((bucket * L2 + i) * L3 + j) * 4) / 64.0;
        }
        output += sum.clamp(0.0, 1.0).powi(2) * read(output_weights + (bucket * L3 + j) * 4) / 64.0;
    }
    output * 400.0
}

#[test]
fn inference_matches_bullet_export() {
    let mut net = Network::new();
    for fen in [
        crate::types::STARTING_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "3N4/b2R2p1/3q3r/6P1/4k1nQ/7B/8/K7 w - - 0 1",
    ] {
        let board = Board::from_fen(fen).unwrap();
        net.full_refresh(&board);
        let accs = &net.stack[net.index].values;
        for stm in [Side::White, Side::Black] {
            for bucket in 0..OUTPUT_BUCKETS {
                let actual = forward::forward(&accs[stm], &accs[!stm], &MODEL, bucket);
                let expected = exported_forward(&accs[stm], &accs[!stm], bucket);
                assert!(
                    (f64::from(actual) - expected).abs() < 1.01,
                    "{fen}, {stm:?}, bucket {bucket}: {actual} vs {expected}"
                );
            }
        }
    }
}
