// Code below was written by AI
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
                let product = simd::shift_right_u16::<9>(simd::mul_low_i16(a, b));
                simd::store_unaligned(paired.as_mut_ptr().add(offset + i).cast(), product);
            }
        }

        let mut first = [zero; L2 / simd::I32_CHUNK];
        for (i, &input) in paired.iter().enumerate() {
            let input = simd::splat_i32(i32::from(input));
            for (block, sum) in first.iter_mut().enumerate() {
                let w = simd::load_i8_to_i32(p.l1_weights[i][bucket].as_ptr().add(block * simd::I32_CHUNK));
                *sum = simd::add_i32(*sum, simd::mul_low_i32(input, w));
            }
        }

        // The dot product is bounded by 1024 * 127 * 128, so i32 is sufficient.
        // Widen before adding the bias, matching the scalar reference for every i32 bias.
        let mut first_sums = [0_i32; L2];
        for (block, sum) in first.into_iter().enumerate() {
            simd::store_unaligned(first_sums.as_mut_ptr().add(block * simd::I32_CHUNK).cast(), sum);
        }
        let mut dual = [0_i32; 2 * L2];
        for j in 0..L2 {
            let s = i64::from(first_sums[j]) + i64::from(p.l1_bias[bucket][j]);
            dual[j] = (s.clamp(0, 16_384) << 5) as i32;
            let t = s.clamp(-16_384, 16_384);
            dual[L2 + j] = ((t * t) >> 9) as i32;
        }

        let mut output = i64::from(p.l3_bias[bucket]);
        for j in (0..L3).step_by(simd::I32_CHUNK) {
            let mut even = zero;
            let mut odd = zero;
            for (i, &input) in dual.iter().enumerate() {
                let input = simd::splat_i32(input);
                let w = simd::load_unaligned(p.l2_weights[i][bucket].as_ptr().add(j).cast());
                // Signed i32 products widened to i64, with separate even/odd neuron lanes.
                even = simd::add_i64(even, simd::mul_even_i32_to_i64(input, w));
                odd = simd::add_i64(odd, simd::mul_even_i32_to_i64(input, simd::shift_right_u64::<32>(w)));
            }
            let mut even_sums = [0_i64; simd::I64_CHUNK];
            let mut odd_sums = [0_i64; simd::I64_CHUNK];
            simd::store_unaligned(even_sums.as_mut_ptr().cast(), even);
            simd::store_unaligned(odd_sums.as_mut_ptr().cast(), odd);
            for k in 0..simd::I32_CHUNK {
                let sum = if k % 2 == 0 { even_sums[k / 2] } else { odd_sums[k / 2] };
                let s = (sum >> 7) + i64::from(p.l2_bias[bucket][j + k]);
                output += s.clamp(0, 262_144) * i64::from(p.l3_weights[j + k][bucket]);
            }
        }

        (output * i64::from(SCALE) / 16_777_216) as i32
    }
}
