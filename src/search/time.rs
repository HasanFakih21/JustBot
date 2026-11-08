use std::time::{Duration, Instant};

use crate::search::data::SearchData;
use crate::tools::parameters::*;

#[derive(Debug, Clone)]
pub struct TimeManager {
    pub clock: Instant,
    pub limit: Limit,
    soft_bound: Option<Duration>,
    hard_bound: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    Soft(u64),
    Hard(u64),
}

#[derive(Debug, Clone)]
pub enum Limit {
    Infinite,
    Exact(u64),
    Nodes(NodeKind),
    Mate(u64),
    Fischer(u64, u64),
    Cyclic(u64, u64, u64),
    Depth(i32),
}

impl TimeManager {
    pub fn new(limit: Limit, full_moves: usize, overhead: u64) -> TimeManager {
        let soft_bound;
        let hard_bound;

        match limit {
            Limit::Fischer(main, inc) => {
                let soft_scale = (tm_base() as f64 / 1000.0
                    - tm_mult() as f64 / 1000.0 * (-(tm_fm_mult() as f64 / 1000.0) * full_moves as f64).exp())
                .max(0.0);
                let hard_scale = tm_hard_scale() as f64 / 1000.0;
                let inc_scale = tm_inc_scale() as f64 / 1000.0;
                let max_time = main.saturating_sub(overhead);

                let soft = (soft_scale * max_time as f64 + inc as f64 * inc_scale) as u64;
                let hard = (hard_scale * max_time as f64 + inc as f64 * inc_scale) as u64;

                soft_bound = Some(Duration::from_millis(soft.min(max_time)));
                hard_bound = Some(Duration::from_millis(hard.min(max_time)));
            }
            Limit::Cyclic(main, inc, moves) => {
                let max_time = main.saturating_sub(overhead);
                let base = (max_time as f64 / moves.max(1) as f64) + inc as f64 * (tm_inc_scale() as f64 / 1000.0);

                soft_bound = Some(Duration::from_millis(
                    ((tm_cyclic_soft_scale() as f64 / 1000.0 * base) as u64).min(max_time),
                ));
                hard_bound = Some(Duration::from_millis(
                    ((tm_cyclic_hard_scale() as f64 / 1000.0 * base) as u64).min(max_time),
                ));
            }
            Limit::Exact(main) => {
                soft_bound = Some(Duration::from_millis(main.saturating_sub(overhead)));
                hard_bound = Some(Duration::from_millis(main.saturating_sub(overhead)));
            }
            _ => {
                soft_bound = None;
                hard_bound = None;
            }
        }

        TimeManager {
            hard_bound,
            soft_bound,
            clock: Instant::now(),
            limit,
        }
    }

    pub fn start_clock(&mut self) {
        self.clock = Instant::now();
    }

    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }

    pub fn soft_limit(&self, data: &SearchData, multiplier: impl Fn() -> f32) -> bool {
        match self.limit {
            Limit::Fischer(_, _) | Limit::Cyclic(_, _, _) if let Some(limit) = self.soft_bound => {
                self.elapsed() >= Duration::from_secs_f32(limit.as_secs_f32() * multiplier())
            }
            Limit::Exact(_) if let Some(limit) = self.soft_bound => self.elapsed() >= limit,
            Limit::Nodes(NodeKind::Soft(limit) | NodeKind::Hard(limit)) => data.nodes() >= limit,
            _ => false,
        }
    }

    pub fn hard_limit(&self, data: &SearchData) -> bool {
        if data.id != 0 || data.root_depth <= 1 {
            return false;
        }

        match self.limit {
            Limit::Fischer(_, _) | Limit::Cyclic(_, _, _) | Limit::Exact(_)
                if let Some(limit) = self.hard_bound
                    && data.nodes().is_multiple_of(2048) =>
            {
                self.elapsed() >= limit
            }
            Limit::Nodes(NodeKind::Hard(limit)) => data.nodes() >= limit,
            Limit::Nodes(NodeKind::Soft(_)) => data.nodes() >= 256_000,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_clock_allocations() {
        for (limit, full_moves, soft, hard) in [
            (Limit::Fischer(10_000, 100), 0, 174, 7537),
            (Limit::Fischer(60_000, 5000), 20, 5858, 48_712),
            (Limit::Cyclic(10_000, 100, 20), 0, 572, 2862),
        ] {
            let time = TimeManager::new(limit, full_moves, 50);
            assert_eq!(time.soft_bound, Some(Duration::from_millis(soft)));
            assert_eq!(time.hard_bound, Some(Duration::from_millis(hard)));
        }
    }

    #[test]
    fn clock_allocations_reserve_overhead_and_cap_increment() {
        for limit in [
            Limit::Fischer(100, 10_000),
            Limit::Cyclic(100, 10_000, 1),
            Limit::Exact(100),
        ] {
            let time = TimeManager::new(limit.clone(), 100, 50);
            assert_eq!(time.soft_bound, Some(Duration::from_millis(50)));
            assert_eq!(time.hard_bound, Some(Duration::from_millis(50)));

            let time = TimeManager::new(limit, 100, 200);
            assert_eq!(time.soft_bound, Some(Duration::ZERO));
            assert_eq!(time.hard_bound, Some(Duration::ZERO));
        }
    }

    #[test]
    fn non_clock_limits_have_no_time_bounds() {
        for limit in [
            Limit::Infinite,
            Limit::Depth(10),
            Limit::Mate(3),
            Limit::Nodes(NodeKind::Soft(1000)),
            Limit::Nodes(NodeKind::Hard(1000)),
        ] {
            let time = TimeManager::new(limit, 20, 50);
            assert_eq!(time.soft_bound, None);
            assert_eq!(time.hard_bound, None);
        }
    }
}
