use std::time::{Duration, Instant};

use crate::{
    search::data::SearchData,
    types::{MAX_PLY, MOVE_OVERHEAD, Side},
};

#[derive(Debug, Clone)]
pub struct TimeManager {
    pub clock: Instant,
    pub settings: TimeSettings,
    pub limits: Limits,
}

// Some settings don't do anything yet
#[derive(Debug, Clone)]
pub struct TimeSettings {
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: u64,
    pub binc: u64,
    pub movestogo: usize,
    pub depth: i32,
    pub nodes: u64,
    pub mate: usize,
    pub movetime: Option<u64>,
}

impl Default for TimeSettings {
    fn default() -> Self {
        TimeSettings {
            wtime: None,
            btime: None,
            winc: 0,
            binc: 0,
            movestogo: 0,
            depth: MAX_PLY as i32 - 1,
            nodes: 0,
            mate: 0,
            movetime: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    soft_time: Option<Duration>,
    hard_time: Option<Duration>,
    depth: i32,
    nodes: Option<u64>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            soft_time: None,
            hard_time: None,
            depth: MAX_PLY as i32 - 1,
            nodes: None,
        }
    }
}

impl TimeManager {
    pub fn new() -> TimeManager {
        TimeManager {
            clock: Instant::now(),
            settings: TimeSettings::default(),
            limits: Limits::default(),
        }
    }

    pub fn clear_limits(&mut self) {
        self.settings = TimeSettings::default();
        self.limits = Limits::default();
    }

    pub fn reset_clock(&mut self) {
        self.clock = Instant::now();
    }

    pub fn set_time_limits(&mut self, side: Side, full_moves: usize) {
        let remaining_time;
        let increment;

        match side {
            Side::White => {
                remaining_time = self.settings.wtime;
                increment = self.settings.winc;
            }

            Side::Black => {
                remaining_time = self.settings.btime;
                increment = self.settings.binc;
            }
        }

        let soft_time;
        let hard_time;

        if let Some(remaining_time) = remaining_time {
            let soft_scale = 0.06 - 0.05 * (-0.035 * full_moves as f64).exp();
            let hard_scale = 0.75;

            let max_time = remaining_time.saturating_sub(MOVE_OVERHEAD);
            let s = (soft_scale * max_time as f64 + increment as f64 * 0.75) as u64;
            let h = (hard_scale * max_time as f64 + increment as f64 * 0.75) as u64;

            soft_time = Some(s.min(max_time));
            hard_time = Some(h.min(max_time));
        } else if let Some(movetime) = self.settings.movetime {
            soft_time = Some(movetime.saturating_sub(MOVE_OVERHEAD));
            hard_time = Some(movetime.saturating_sub(MOVE_OVERHEAD));
        } else {
            soft_time = None;
            hard_time = None;
        }

        self.limits.soft_time = soft_time.map(Duration::from_millis);
        self.limits.hard_time = hard_time.map(Duration::from_millis);
    }

    pub fn set_depth_limit(&mut self) {
        self.limits.depth = self.settings.depth;
    }

    pub fn set_nodes_limit(&mut self) {
        self.limits.nodes = Some(self.settings.nodes);
    }

    pub fn node_limit(&self) -> Option<u64> {
        self.limits.nodes
    }

    pub fn depth_limit(&self) -> i32 {
        self.limits.depth
    }

    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }

    pub fn soft_limit(&self, multiplier: impl Fn() -> f32) -> bool {
        if let Some(soft_limit) = self.limits.soft_time {
            self.elapsed() >= Duration::from_secs_f32(soft_limit.as_secs_f32() * multiplier())
        } else {
            false
        }
    }

    pub fn hard_limit(&self, data: &SearchData) -> bool {
        if !data.nodes().is_multiple_of(2048) || data.id != 0 || data.root_depth <= 1 {
            return false;
        }

        if let Some(hard_limit) = self.limits.hard_time {
            self.elapsed() > hard_limit
        } else {
            false
        }
    }
}

impl Default for TimeManager {
    fn default() -> Self {
        Self::new()
    }
}
