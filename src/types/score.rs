use crate::search::data::SearchData;

pub struct Score;

impl Score {
    pub const INFINITY: i32 = 100000;
    pub const MATE: i32 = 9000;
    pub const MATE_CUTOFF: i32 = 8900;
    pub const TIMEOUT: i32 = 111111;
}

pub const fn mated(score: i32) -> bool {
    score <= -Score::MATE_CUTOFF
}

pub const fn mating(score: i32) -> bool {
    score >= Score::MATE_CUTOFF
}

pub fn is_draw(data: &SearchData) -> bool {
    if data.board.state.half_move_clock > 4 {
        //50 move rule
        if data.board.state.half_move_clock >= 100 {
            return true;
        }

        //We need to check history if positions were repeated only for the side to move.
        let count = data.board.detect_repetitions();
        if count >= 2 {
            return true;
        }
    }

    false
}