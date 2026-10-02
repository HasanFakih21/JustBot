use crate::{
    nnue::{
        Aligned, L1, L2, L3, Parameters, Q, Q0, SCALE,
        accumulator::Accumulator,
        simd::{self},
    },
    types::Side,
};

pub fn activate_features(acc: &Accumulator, stm: Side) -> Aligned<[u8; L1]> {
    let mut output = Aligned::new([0; L1]);

    let zero = simd::splat_i16(0);
    let one = simd::splat_i16(Q0 as i16);

    unsafe {
        for flip in [0, 1] {
            let input = &acc.values[stm as usize ^ flip];

            for i in (0..L1 / 2).step_by(2 * simd::I16_CHUNK) {
                let lhs1 = *input.as_ptr().add(i).cast();
                let lhs2 = *input.as_ptr().add(i + simd::I16_CHUNK).cast();

                let rhs1 = *input.as_ptr().add(i + L1 / 2).cast();
                let rhs2 = *input.as_ptr().add(i + L1 / 2 + simd::I16_CHUNK).cast();

                let lhs1_clipped = simd::clamp_i16(lhs1, zero, one);
                let lhs2_clipped = simd::clamp_i16(lhs2, zero, one);

                let rhs1_clipped = simd::clamp_i16(rhs1, zero, one);
                let rhs2_clipped = simd::clamp_i16(rhs2, zero, one);

                let shifted1 = simd::shift_left_i16::<7>(lhs1_clipped);
                let shifted2 = simd::shift_left_i16::<7>(lhs2_clipped);

                let product1 = simd::mul_high_i16(shifted1, rhs1_clipped);
                let product2 = simd::mul_high_i16(shifted2, rhs2_clipped);

                let packed = simd::packus(product1, product2);
                let unpacked = simd::permute(packed);

                *output.as_mut_ptr().add(i + flip * L1 / 2).cast() = unpacked;
            }
        }
    }

    output
}

pub fn propogate_l1(ft_out: &Aligned<[u8; L1]>, bucket: usize, parameters: &Parameters) -> Aligned<[i32; L2]> {
    const CHUNKS: usize = 4;
    const L2_LANES: usize = L2 / simd::I32_CHUNK;

    let mut pre_activations = Aligned::new([simd::zeroed(); L2_LANES]);

    unsafe {
        let packed = std::slice::from_raw_parts(ft_out.as_ptr().cast::<i32>(), L1 / CHUNKS);

        #[cfg(target_feature = "avx512vnni")]
        let mut pre_b = Aligned::new([simd::zeroed(); L2_LANES]);

        let mut pairs = packed.chunks_exact(2);

        for (pair_index, pair) in (&mut pairs).enumerate() {
            let index1 = pair_index * 2;
            let index2 = index1 + 1;

            let input1 = simd::splat_i32(*pair.get_unchecked(0));
            let input2 = simd::splat_i32(*pair.get_unchecked(1));

            let weights1 = parameters.l1_weights[bucket].as_ptr().add(index1 * L2 * CHUNKS);
            let weights2 = parameters.l1_weights[bucket].as_ptr().add(index2 * L2 * CHUNKS);

            for j in (0..L2).step_by(simd::I32_CHUNK) {
                let weights1 = *weights1.add(j * CHUNKS).cast();
                let weights2 = *weights2.add(j * CHUNKS).cast();

                let lane = j / simd::I32_CHUNK;
                #[cfg(target_feature = "avx512vnni")]
                {
                    pre_activations[lane] = simd::dpbusd(pre_activations[lane], input1, weights1);
                    pre_b[lane] = simd::dpbusd(pre_b[lane], input2, weights2);
                }

                #[cfg(not(target_feature = "avx512vnni"))]
                {
                    pre_activations[lane] =
                        simd::double_dpbusd(pre_activations[lane], input1, weights1, input2, weights2);
                }
            }
        }

        #[cfg(target_feature = "avx512vnni")]
        for lane in 0..L2_LANES {
            pre_activations[lane] = simd::add_i32(pre_activations[lane], pre_b[lane]);
        }

        if let Some(last) = pairs.remainder().first() {
            let index = packed.len() - 1;
            let input = simd::splat_i32(*last);
            let weights = parameters.l1_weights[bucket].as_ptr().add(index * L2 * CHUNKS);

            for j in (0..L2).step_by(simd::I32_CHUNK) {
                let weights = *weights.add(j * CHUNKS).cast();
                let lane = j / simd::I32_CHUNK;
                pre_activations[lane] = simd::dpbusd(pre_activations[lane], input, weights);
            }
        }

        let mut output = Aligned::new([0; L2]);
        let zero = simd::zeroed();
        let maximum = simd::splat_i32(Q as i32 * 256);

        for i in (0..L2).step_by(simd::I32_CHUNK) {
            let biases = *parameters.l1_bias[bucket].as_ptr().add(i).cast();
            let vector = simd::add_i32(pre_activations[i / simd::I32_CHUNK], biases);

            let clipped = simd::clamp_i32(vector, zero, maximum);
            let squared = simd::mul_low_i32(clipped, clipped);

            *output.as_mut_ptr().add(i).cast() = simd::shift_right_i32::<16>(squared);
        }

        output
    }
}

pub fn propogate_l2(l1_out: &Aligned<[i32; L2]>, bucket: usize, parameters: &Parameters) -> Aligned<[i32; L3]> {
    let mut output = Aligned::new(parameters.l2_bias[bucket]);

    unsafe {
        for i in 0..L2 {
            let input = simd::splat_i32(l1_out[i]);
            let weights = parameters.l2_weights[bucket].as_ptr().add(i * L3);

            for j in (0..L3).step_by(simd::I32_CHUNK) {
                let weights = *weights.add(j).cast();
                let vector = output.as_mut_ptr().add(j).cast();
                *vector = simd::mul_add_i32(weights, input, *vector);
            }
        }

        let zero = simd::zeroed();
        let maximum = simd::splat_i32((Q as i32).pow(3));

        for i in (0..L3).step_by(simd::I32_CHUNK) {
            let vector = output.as_mut_ptr().add(i).cast();
            *vector = simd::clamp_i32(*vector, zero, maximum);
        }
    }

    output
}

pub fn propogate_l3(l2_out: &Aligned<[i32; L3]>, bucket: usize, parameters: &Parameters) -> i32 {
    const LANES: usize = 16 / simd::I64_CHUNK;

    let input = l2_out.as_ptr();
    let weights = parameters.l3_weights[bucket].as_ptr();

    let mut output = [simd::zeroed(); LANES];

    unsafe {
        for (lane, result) in output.iter_mut().enumerate() {
            for i in (0..L3).step_by(LANES * simd::I64_CHUNK) {
                let a = simd::load_i32_as_i64(weights.add(i + lane * simd::I64_CHUNK));
                let b = simd::load_i32_as_i64(input.add(i + lane * simd::I64_CHUNK));

                *result = simd::mul_add_i32_to_i64(a, b, *result);
            }
        }
    }

    let sum = simd::horizontal_sum_i64(output) + i64::from(parameters.l3_bias[bucket]);
    ((sum * i64::from(SCALE)) / (Q as i64).pow(4)) as i32
}
