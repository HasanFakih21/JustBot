use crate::nnue::{L1, L2, L3, Parameters, QA, SCALE, accumulator::Accumulator};

pub(crate) fn forward(us: &Accumulator, them: &Accumulator, p: &Parameters, bucket: usize) -> i32 {
    let mut paired = [0_i32; L1];
    for (offset, acc) in [(0, us), (L1 / 2, them)] {
        for i in 0..L1 / 2 {
            let a = i32::from(acc.vals[i]).clamp(0, i32::from(QA));
            let b = i32::from(acc.vals[i + L1 / 2]).clamp(0, i32::from(QA));
            paired[offset + i] = (a * b) >> 8;
        }
    }

    let mut hidden = [0.0_f32; L2];
    for j in 0..L2 {
        let mut sum = i64::from(p.l1_bias[bucket][j]);
        for i in 0..L1 {
            sum += i64::from(paired[i]) * i64::from(p.l1_weights[bucket][i][j]);
        }
        // Export compensates for (255 / 256)^2 in l1/w; the sum scale is 256 * 128.
        let clipped = (sum as f32 / 32768.0).clamp(0.0, 1.0);
        hidden[j] = clipped * clipped;
    }

    let mut output = p.l3_bias[bucket] as f32 / 4096.0;
    for j in 0..L3 {
        let mut sum = p.l2_bias[bucket][j] as f32 / 4096.0;
        for i in 0..L2 {
            sum += hidden[i] * (p.l2_weights[bucket][i][j] as f32 / 64.0);
        }
        let clipped = sum.clamp(0.0, 1.0);
        output += clipped * clipped * (p.l3_weights[bucket][j] as f32 / 64.0);
    }
    (output * SCALE as f32) as i32
}
