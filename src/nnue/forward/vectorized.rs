use crate::nnue::{L1, L2, L3, Parameters, QA, SCALE, accumulator::Accumulator, simd};

pub(crate) fn forward(us: &Accumulator, them: &Accumulator, p: &Parameters, bucket: usize) -> i32 {
    // Buffer loops use the selected backend's lane counts and unaligned loads/stores.
    unsafe {
        let zero = simd::zeroed();
        let hi = simd::splat_i16(QA);
        let mut paired = [0_i16; L1];
        for (offset, acc) in [(0, us), (L1 / 2, them)] {
            for i in (0..L1 / 2).step_by(simd::I16_CHUNK) {
                let a = simd::load_unaligned(acc.vals.as_ptr().add(i).cast());
                let b = simd::load_unaligned(acc.vals.as_ptr().add(i + L1 / 2).cast());
                let a = simd::clamp_i16(a, zero, hi);
                let b = simd::clamp_i16(b, zero, hi);
                // Products fit unsigned u16. A logical shift preserves values above i16::MAX.
                let product = simd::shift_right_u16::<8>(simd::mul_low_i16(a, b));
                simd::store_unaligned(paired.as_mut_ptr().add(offset + i).cast(), product);
            }
        }

        let mut first = [zero; L2 / simd::I32_CHUNK];
        for (i, &input) in paired.iter().enumerate() {
            if input == 0 {
                continue;
            }
            let input = simd::splat_i32(i32::from(input));
            for (block, sum) in first.iter_mut().enumerate() {
                let w = simd::load_i8_to_i32(p.l1_weights[bucket][i].as_ptr().add(block * simd::I32_CHUNK));
                *sum = simd::add_i32(*sum, simd::mul_low_i32(input, w));
            }
        }

        // The dot product is bounded by 768 * 254 * 128, so i32 is sufficient.
        // Widen before adding the bias, matching the scalar reference for every i32 bias.
        let mut first_sums = [0_i32; L2];
        for (block, sum) in first.into_iter().enumerate() {
            simd::store_unaligned(first_sums.as_mut_ptr().add(block * simd::I32_CHUNK).cast(), sum);
        }
        let mut hidden = [0.0_f32; L2];
        for j in 0..L2 {
            let sum = i64::from(first_sums[j]) + i64::from(p.l1_bias[bucket][j]);
            let clipped = (sum as f32 / 32768.0).clamp(0.0, 1.0);
            hidden[j] = clipped * clipped;
        }

        // Keep the small dense layers in floating point, avoiding another activation
        // quantisation. The exported weights use Q=64 and biases use Q^2=4096.
        let mut sums = [0.0_f32; L3];
        for j in (0..L3).step_by(simd::I32_CHUNK) {
            let mut sum = simd::mul_f32(
                simd::i32_to_f32(simd::load_unaligned(p.l2_bias[bucket].as_ptr().add(j).cast())),
                simd::splat_f32(1.0 / 4096.0),
            );
            for (i, &input) in hidden.iter().enumerate() {
                let weights = simd::mul_f32(
                    simd::i32_to_f32(simd::load_unaligned(p.l2_weights[bucket][i].as_ptr().add(j).cast())),
                    simd::splat_f32(1.0 / 64.0),
                );
                sum = simd::add_f32(sum, simd::mul_f32(simd::splat_f32(input), weights));
            }
            simd::store_f32(sums.as_mut_ptr().add(j), sum);
        }
        let mut output = p.l3_bias[bucket] as f32 / 4096.0;
        for (j, sum) in sums.into_iter().enumerate() {
            let clipped = sum.clamp(0.0, 1.0);
            output += clipped * clipped * (p.l3_weights[bucket][j] as f32 / 64.0);
        }
        (output * SCALE as f32) as i32
    }
}
