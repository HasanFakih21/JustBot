use crate::{
    nnue::{Aligned, L1, L2, L3, Parameters, Q, Q0, SCALE, accumulator::Accumulator},
    types::Side,
};

pub fn activate_features(acc: &Accumulator, stm: Side) -> Aligned<[u8; L1]> {
    let mut output = Aligned::new([0; L1]);

    for flip in [0, 1] {
        let input = &acc.values[stm as usize ^ flip];

        for i in 0..L1 / 2 {
            let left = i32::from(input[i]).clamp(0, Q0 as i32);
            let right = i32::from(input[i + L1 / 2]).clamp(0, Q0 as i32);

            output[i + flip * L1 / 2] = (left * right >> 9) as u8;
        }
    }

    output
}

pub fn propogate_l1(ft_out: &Aligned<[u8; L1]>, bucket: usize, parameters: &Parameters) -> Aligned<[i32; L2]> {
    const CHUNKS: usize = 4;

    let mut pre_activations = parameters.l1_bias[bucket];
    let weights = &parameters.l1_weights[bucket];

    for (index, input) in ft_out.chunks_exact(CHUNKS).enumerate() {
        let base = index * L2 * CHUNKS;

        for i in 0..L2 {
            for j in 0..CHUNKS {
                pre_activations[i] += i32::from(input[j]) * i32::from(weights[base + i * CHUNKS + j]);
            }
        }
    }

    let mut output = Aligned::new([0; L2]);
    for i in 0..L2 {
        let clipped = pre_activations[i].clamp(0, (Q as i32) * 256);
        output[i] = (clipped * clipped) >> 16;
    }

    output
}

pub fn propogate_l2(l1_out: &Aligned<[i32; L2]>, bucket: usize, parameters: &Parameters) -> Aligned<[i32; L3]> {
    let mut output = Aligned::new(parameters.l2_bias[bucket]);
    let weights = &parameters.l2_weights[bucket];

    for i in 0..L2 {
        for j in 0..L3 {
            output[j] += l1_out.data[i] * weights[i * L3 + j];
        }
    }

    for i in 0..L3 {
        output[i] = output[i].clamp(0, (Q as i32).pow(3));
    }

    output
}

pub fn propogate_l3(l2_out: &Aligned<[i32; L3]>, bucket: usize, parameters: &Parameters) -> i32 {
    let mut sum = i64::from(parameters.l3_bias[bucket]);
    let weights = &parameters.l3_weights[bucket];

    for i in 0..L3 {
        sum += i64::from(l2_out[i]) * i64::from(weights[i]);
    }

    ((sum * i64::from(SCALE)) / (Q as i64).pow(4)) as i32
}
