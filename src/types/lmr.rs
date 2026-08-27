use crate::{tools::parameters::*, types::zeroed_box};

/// `[Is Quiet][Depth][Move Count]`
pub struct LMRTable {
    pub base: Box<[[[i32; 64]; 128]; 2]>,
}

impl LMRTable {
    pub fn init(&mut self) {
        let lmr_quiet_base = lmr_quiet_base() as f32 / 10000.0;
        let lmr_noisy_base = lmr_noisy_base() as f32 / 10000.0;
        let lmr_quiet_div = lmr_quiet_div() as f32 / 10000.0;
        let lmr_noisy_div = lmr_noisy_div() as f32 / 10000.0;

        for depth in 1..128 {
            for move_count in 1..64 {
                // Quiet Moves
                self.base[1][depth][move_count] = ((lmr_quiet_base
                    + (move_count as f32).ln() * (depth as f32).ln() / lmr_quiet_div)
                    * 1024.0) as i32;
                // Noisy Moves
                self.base[0][depth][move_count] = ((lmr_noisy_base
                    + (move_count as f32).ln() * (depth as f32).ln() / lmr_noisy_div)
                    * 1024.0) as i32;
            }
        }
    }
}

impl Default for LMRTable {
    fn default() -> Self {
        Self { base: zeroed_box() }
    }
}
