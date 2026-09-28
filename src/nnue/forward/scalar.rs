use crate::nnue::{L1, L2, L3, Parameters, QA, SCALE, accumulator::Accumulator};

// Reference implementation: keep wide intermediates and exactly the same shifts as SIMD.
pub(crate) fn forward(us: &Accumulator, them: &Accumulator, p: &Parameters, bucket: usize) -> i32 {
    let mut paired = [0_i64; L1];
    for (offset, acc) in [(0, us), (L1 / 2, them)] {
        for i in 0..L1 / 2 {
            let a = i64::from(acc.vals[i]).clamp(0, i64::from(QA));
            let b = i64::from(acc.vals[i + L1 / 2]).clamp(0, i64::from(QA));
            paired[offset + i] = (a * b) >> 9;
        }
    }

    let mut dual = [0_i64; 2 * L2];
    for j in 0..L2 {
        let mut s = i64::from(p.l1_bias[bucket][j]);
        for i in 0..L1 {
            s += paired[i] * i64::from(p.l1_weights[i][bucket][j]);
        }
        dual[j] = s.clamp(0, 16_384) << 5;
        let t = s.clamp(-16_384, 16_384);
        dual[L2 + j] = (t * t) >> 9;
    }

    let mut output = i64::from(p.l3_bias[bucket]);
    for j in 0..L3 {
        let mut s = 0_i64;
        for i in 0..2 * L2 {
            s += dual[i] * i64::from(p.l2_weights[i][bucket][j]);
        }
        s = (s >> 7) + i64::from(p.l2_bias[bucket][j]);
        let activated = s.clamp(0, 262_144);
        output += activated * i64::from(p.l3_weights[j][bucket]);
    }

    (output * i64::from(SCALE) / 16_777_216) as i32
}
