use std::time::{Duration, Instant};

use crate::types::{MAX_PLY, Side};

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
    pub movetime: u64,
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
            movetime: 0,
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

    pub fn set_time_limits(&mut self, side: Side) {
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

        let Some(remaining_time) = remaining_time else {
            return;
        };

        // Simple time managment strategy: remaining time / 20 + increment / 2
        self.limits.soft_time = Some(Duration::from_millis(
            (remaining_time / 20) + (increment / 2),
        ));
        self.limits.hard_time = Some(Duration::from_millis(
            (remaining_time / 2) + (increment / 2),
        ));
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

    pub fn soft_limit(&self) -> bool {
        if let Some(soft_limit) = self.limits.soft_time {
            self.elapsed() >= soft_limit
        } else {
            false
        }
    }

    pub fn hard_limit(&self, nodes: u64, id: usize) -> bool {
        if !nodes.is_multiple_of(2048) || id != 0 {
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
