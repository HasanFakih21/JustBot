use crate::{
    board::{movegen::MoveGenKind, see},
    search::data::SearchData,
    types::{Move, MoveEntry, MoveList, stackvec::StackVec},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    HashMove,
    FirstNoisy,
    GoodNoisy,
    Quiet,
    BadNoisy,
}

#[derive(Debug)]
pub struct MovePicker {
    moves: MoveList,
    tt_move: Option<Move>,
    status: Status,
    bad_noisy: StackVec<Move, 256>,
    bad_index: usize,
    noisy_count: usize,
}

impl MovePicker {
    pub fn new(tt_move: Option<Move>) -> MovePicker {
        Self {
            moves: MoveList::new(),
            tt_move,
            status: if tt_move.is_some() {
                Status::HashMove
            } else {
                Status::FirstNoisy
            },
            bad_noisy: StackVec::new(),
            bad_index: 0,
            noisy_count: 0,
        }
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn next(&mut self, data: &SearchData, skip_quiets: bool, ply: isize) -> Option<Move> {
        let board = &data.board;
        if self.status == Status::HashMove {
            self.status = Status::FirstNoisy;
            let tt_move = self.tt_move.unwrap();
            if (!skip_quiets || !tt_move.get_kind().is_quiet()) && data.board.is_legal(tt_move) {
                return Some(tt_move);
            }
        }

        if self.status == Status::FirstNoisy {
            board.append_moves(MoveGenKind::Noisy, &mut self.moves);
            self.remove_tt_move();
            self.score_noisy_moves(data);
            self.status = Status::GoodNoisy;
        }

        if self.status == Status::GoodNoisy {
            while !self.moves.is_empty() {
                let best_entry = self.best_entry();
                let threshold = -best_entry.score / 4 + 75;
                if !data.board.see(best_entry.mv, threshold) {
                    self.bad_noisy.push(best_entry.mv);
                    continue;
                }

                self.noisy_count += 1;
                return Some(best_entry.mv);
            }

            if !skip_quiets {
                self.status = Status::Quiet;
                board.append_moves(MoveGenKind::Quiet, &mut self.moves);
                self.remove_tt_move();
                self.score_quiet_moves(data, ply);
            } else {
                self.status = Status::BadNoisy;
            }
        }

        if self.status == Status::Quiet && !skip_quiets {
            if !self.moves.is_empty() {
                return Some(self.best_entry().mv);
            }

            self.status = Status::BadNoisy;
        }

        //Bad Noisy
        if self.bad_index < self.bad_noisy.len() {
            let m = self.bad_noisy.get(self.bad_index);
            self.bad_index += 1;
            return Some(m);
        }

        None
    }

    fn score_noisy_moves(&mut self, data: &SearchData) {
        let threats = data.board.state.threats;
        for entry in self.moves.iter_mut() {
            let mv = entry.mv;
            let mut score = 0;

            //Bonus for promotions
            if mv.get_kind().is_queen_promotion() {
                score += 5000;
            }

            let piece = data.board.get_piece_at_square(mv.get_from());
            let to = mv.get_to();
            let captured = data
                .board
                .get_piece_at_square(mv.get_capture_square())
                .map(|e| e.1);
            if let Some(p) = captured {
                score += see::value(p)
            }

            score += data.noisy_history.get(piece, to, captured, threats) / 8;
            entry.score = score;
        }
    }

    fn score_quiet_moves(&mut self, data: &SearchData, ply: isize) {
        let side = data.board.state.side_to_move;
        let threats = data.board.state.threats;

        for entry in self.moves.iter_mut() {
            let mv = entry.mv;
            let conthistory_score = 1600 * data.get_conthistory(mv, ply, 1) / 1024
                + 1000 * data.get_conthistory(mv, ply, 2) / 1024;

            entry.score = data.quiet_history.get(threats, side, mv)
                + conthistory_score
                + (10000 * data.board.is_direct_check(mv) as i32);
        }
    }

    fn best_entry(&mut self) -> MoveEntry {
        let mut best_index = 0;
        let mut best_score = i32::MIN;

        for (index, entry) in self.moves.iter().enumerate() {
            if entry.score >= best_score {
                best_score = entry.score;
                best_index = index;
            }
        }

        self.moves.remove(best_index).unwrap()
    }

    fn remove_tt_move(&mut self) {
        if let Some(tt_mv) = self.tt_move
            && let Some(index) = self.moves.iter().position(|e| tt_mv == e.mv)
        {
            self.moves.remove(index);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        board::Board,
        search::{data::SearchData, movepicker::MovePicker},
    };

    #[test]
    fn test_move_picker() {
        let data = SearchData {
            board: Board::from_fen(
                "r1bqk2r/ppp1p1pp/3p2n1/3P4/4PN2/5b2/PPPP2Pp/RNBQK1R1 b Qkq - 0 1",
            )
            .unwrap(),
            ..Default::default()
        };

        let mut move_picker = MovePicker::new(None);
        println!("{}", move_picker.moves);
        while let Some(m) = move_picker.next(&data, true, 0) {
            print!("{m}: ");
            print!(
                "Value: {}, Value: {}",
                if m.is_capture() {
                    data.board.capture_move_value(m)
                } else {
                    2000
                },
                (data.board.move_value(m) - data.board.move_loss(m))
            );
            println!();
        }
    }
}
