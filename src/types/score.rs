use crate::{search::data::SearchData, types::MAX_PLY};

pub struct Score;

impl Score {
    pub const NONE: i32 = 32001;
    pub const INFINITY: i32 = 32000;
    pub const MATE: i32 = 30000;
    pub const MATE_CUTOFF: i32 = Score::MATE - MAX_PLY as i32;
    pub const TIMEOUT: i32 = 111111;
    pub const DRAW: i32 = 0;

    pub fn draw(data: &SearchData) -> i32 {
        Score::DRAW - 1 + (data.nodes() & 0x2) as i32
    }
}

pub const fn is_loss(score: i32) -> bool {
    score <= -Score::MATE_CUTOFF
}

pub const fn is_win(score: i32) -> bool {
    score >= Score::MATE_CUTOFF
}

pub const fn is_decisive(score: i32) -> bool {
    score.abs() >= Score::MATE_CUTOFF
}

pub const fn from_tt(score: i16, ply: isize) -> i16 {
    if is_decisive(score as i32) && score as i32 != Score::NONE {
        score - (score.signum() * ply as i16)
    } else {
        score
    }
}

pub const fn ilerp<const K: i32>(a: i32, b: i32, t: i32) -> i32 {
    (a * (K - t) + b * t) / K
}
