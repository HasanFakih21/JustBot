use crate::{
    tools::parameters::{lmr_base, lmr_noisy_div, lmr_quiet_div},
    types::allocate_zeroed_box,
};

/// `[Is Quiet][Depth][Move Count]`
pub struct LMRTable {
    pub base: Box<[[[i32; 64]; 128]; 2]>,
}

impl LMRTable {
    pub fn init(&mut self) { 
        let lmr_base = lmr_base() as f32 / 10000.0;
        let lmr_quiet_div = lmr_quiet_div() as f32 / 10000.0;
        let lmr_noisy_div = lmr_noisy_div() as f32 / 10000.0;

        for depth in 0..128 {
            for move_count in 0..64 {
                let reduction = lmr_base + f32::ln(depth as f32) * f32::ln(move_count as f32);

                self.base[1][depth][move_count] = ((reduction / lmr_quiet_div) * 1024.0) as i32;
                self.base[0][depth][move_count] = ((reduction / lmr_noisy_div) * 1024.0) as i32;
            }
        }
    }
}

impl Default for LMRTable {
    fn default() -> Self {
        Self {
            base: unsafe { allocate_zeroed_box() },
        }
    }
}
