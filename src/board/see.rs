use crate::{
    attacks::RAYS,
    board::Board,
    types::{BitBoard, Move, Piece, Side, Square},
};

impl Board {
    pub fn see(&self, m: Move, threshold: i32) -> bool {
        if (m.is_promotion() && !m.is_capture()) || m.is_castling() {
            return true;
        }

        let mut balance = self.move_value(m) - threshold;

        if balance < 0 {
            return false;
        }

        balance -= self.move_loss(m);

        if balance >= 0 {
            return true;
        }

        let mut occupancies = self.get_all_occupancy();
        occupancies.clear_bit(m.get_from());

        let to = m.get_to();

        if m.is_en_passant() {
            occupancies.clear_bit(to ^ 8);
        }

        let diagonals =
            self.state.pieces[Piece::Bishop as usize] | self.state.pieces[Piece::Queen as usize];
        let orthogonals =
            self.state.pieces[Piece::Rook as usize] | self.state.pieces[Piece::Queen as usize];

        let king_rays = [
            RAYS[to as usize][self.get_king_square(Side::White) as usize],
            RAYS[to as usize][self.get_king_square(Side::Black) as usize],
        ];

        let mut attackers = self.attackers_to(to, occupancies) & occupancies;
        let mut stm = self.state.side_to_move.other();

        loop {
            let mut our_attackers = attackers & self.state.occupancies[stm as usize];

            if !((self.state.pinners[stm.other() as usize] & occupancies).is_empty()) {
                our_attackers &= !(self.state.pinned[stm as usize] & !king_rays[stm as usize]);
            }

            if our_attackers.is_empty() {
                break;
            }

            let attacker = self.least_valuable_attacker(our_attackers);

            //Makes sure the king can't capture a defended piece
            if attacker == Piece::King
                && !(attackers & self.state.occupancies[stm.other() as usize]).is_empty()
            {
                break;
            }

            occupancies.clear_bit(
                (self.state.pieces[attacker as usize] & our_attackers)
                    .least_sig_bit()
                    .unwrap(),
            );

            stm = stm.other();
            balance = -balance - 1 - attacker.value();

            if balance >= 0 {
                break;
            }

            //Update possble revealed sliding attackers
            if matches!(attacker, Piece::Bishop | Piece::Queen | Piece::Pawn) {
                attackers |= self.get_bishop_attacks(to, occupancies) & diagonals;
            }

            if matches!(attacker, Piece::Rook | Piece::Queen) {
                attackers |= self.get_rook_attacks(to, occupancies) & orthogonals;
            }

            attackers &= occupancies;
        }

        stm != self.state.side_to_move
    }

    pub const fn move_loss(&self, m: Move) -> i32 {
        if m.is_promotion() {
            return unsafe { m.get_promoted_piece().unwrap_unchecked().value() };
        }

        if let Some((_, piece)) = self.get_piece_at_square(m.get_from()) {
            return piece.value();
        }

        0
    }

    pub const fn move_value(&self, m: Move) -> i32 {
        if let Some((_, piece)) = self.get_piece_at_square(m.get_capture_square()) {
            let mut value = piece.value();
            if let Some(promotion_piece) = m.get_promoted_piece() {
                value += promotion_piece.value() - Piece::Pawn.value();
            }

            return value;
        }

        0
    }

    pub const fn capture_move_value(&self, mv: Move) -> i32 {
        let attacker = self.get_piece_at_square(mv.get_from()).unwrap().1;
        let victim = self.get_piece_at_square(mv.get_capture_square()).unwrap().1;

        victim.value() - attacker.value()
    }

    pub fn least_valuable_attacker(&self, attackers: BitBoard) -> Piece {
        if !(attackers & self.state.pieces[Piece::Pawn as usize]).is_empty() {
            return Piece::Pawn;
        }

        if !(attackers & self.state.pieces[Piece::Knight as usize]).is_empty() {
            return Piece::Knight;
        }

        if !(attackers & self.state.pieces[Piece::Bishop as usize]).is_empty() {
            return Piece::Bishop;
        }

        if !(attackers & self.state.pieces[Piece::Rook as usize]).is_empty() {
            return Piece::Rook;
        }

        if !(attackers & self.state.pieces[Piece::Queen as usize]).is_empty() {
            return Piece::Queen;
        }

        if !(attackers & self.state.pieces[Piece::King as usize]).is_empty() {
            return Piece::King;
        }

        unreachable!("No attackers");
    }

    pub fn attackers_to(&self, square: Square, occupancies: BitBoard) -> BitBoard {
        let diagonals =
            self.state.pieces[Piece::Bishop as usize] | self.state.pieces[Piece::Queen as usize];
        let orthogonals =
            self.state.pieces[Piece::Rook as usize] | self.state.pieces[Piece::Queen as usize];

        (self.get_bishop_attacks(square, occupancies) & diagonals)
            | (self.get_rook_attacks(square, occupancies) & orthogonals)
            | (self.get_pawn_attacks(square, Side::White) & self.state.pieces[Piece::Pawn as usize])
            | (self.get_pawn_attacks(square, Side::Black) & self.state.pieces[Piece::Pawn as usize])
            | (self.get_knight_attacks(square) & self.state.pieces[Piece::Knight as usize])
            | (self.get_king_attacks(square) & self.state.pieces[Piece::King as usize])
    }
}

pub fn value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 430,
        Piece::Bishop => 460,
        Piece::Rook => 650,
        Piece::Queen => 1300,
        Piece::King => 0,
    }
}

#[cfg(test)]
mod tests {
    use crate::search::data::SearchData;

    use super::*;

    #[test]
    fn test_see() {
        let data = SearchData {
            board: Board::from_fen("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1")
                .unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("e2e5").unwrap();
        assert!(!data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1")
                .unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("d3e5").unwrap();
        assert!(!data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - -").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("e1e5").unwrap();
        assert!(data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("1k1r3q/1pp4p/pn3b2/4p3/P7/3N2P1/1PP1R1BP/2K1Q3 w - - 1 2")
                .unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("d3e5").unwrap();
        assert!(data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("1k1r3q/1pp5/pn6/4R3/P7/5B2/1PP2Q1p/2K5 b - - 1 7").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("h2h1q").unwrap();
        assert!(data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("1k5q/1pp5/pn6/3r4/P7/5B2/1PP2Q1p/2K3R1 b - - 5 9").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("h2g1q").unwrap();
        assert!(data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen(
                "r1bqk2r/ppp1p1pp/3p2n1/3P4/4PN2/5b2/PPPP2Pp/RNBQK1R1 b Qkq - 0 1",
            )
            .unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("h2g1q").unwrap();
        assert!(data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("4k3/4n3/8/5p2/8/5Q2/8/K3R3 w - - 0 1").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("f3f5").unwrap();
        assert!(data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("7K/8/8/8/8/2p5/8/knR5 w - - 0 1").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("c1c3").unwrap();
        assert!(!data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("7k/8/8/8/r7/r1p5/N7/K3B3 w - - 0 1").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("e1c3").unwrap();
        assert!(!data.board.see(m, -150));
    }
}
