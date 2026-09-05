use crate::{
    attacks::RAYS,
    board::Board,
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks},
    types::{BitBoard, Move, OptionPiece, Piece, Side, Square},
};

impl Board {
    pub fn see(&self, m: Move, threshold: i32) -> bool {
        if m.is_promotion() && !m.is_capture() {
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

        let mut occupancies = self.all_occupancy();
        occupancies.clear_bit(m.from());

        if m.is_en_passant() {
            occupancies.clear_bit(m.to() ^ 8);
        }

        let mut attackers = self.attackers_to(m.to(), occupancies) & occupancies;
        let mut stm = !self.state.side_to_move;

        let diagonals = self.state.pieces[Piece::Bishop] | self.state.pieces[Piece::Queen];
        let orthogonals = self.state.pieces[Piece::Rook] | self.state.pieces[Piece::Queen];

        let king_rays = [
            RAYS[m.to()][self.king_square(Side::White)],
            RAYS[m.to()][self.king_square(Side::Black)],
        ];

        loop {
            let mut our_attackers = attackers & self.occ(stm);
            if !((self.state.pinners[!stm] & occupancies).is_empty()) {
                our_attackers &= !(self.state.pinned[stm] & !king_rays[stm]);
            }

            if our_attackers.is_empty() {
                break;
            }

            let attacker = self.least_valuable_attacker(our_attackers);

            // Makes sure the king can't capture a defended piece
            if attacker == Piece::King && !(attackers & self.occ(!stm)).is_empty() {
                break;
            }

            occupancies.clear_bit((self.state.pieces[attacker] & our_attackers).least_sig_bit().unwrap());

            stm = !stm;
            balance = -balance - 1 - attacker.value();

            if balance >= 0 {
                break;
            }

            // Update possble revealed sliding attackers
            if matches!(attacker, Piece::Bishop | Piece::Queen | Piece::Pawn) {
                attackers |= bishop_attacks(m.to(), occupancies) & diagonals;
            }

            if matches!(attacker, Piece::Rook | Piece::Queen) {
                attackers |= rook_attacks(m.to(), occupancies) & orthogonals;
            }

            attackers &= occupancies;
        }

        stm != self.state.side_to_move
    }

    fn move_loss(&self, m: Move) -> i32 {
        if m.is_promotion() {
            return unsafe { m.promoted_piece().unwrap_unchecked().value() };
        }

        if let OptionPiece::Some(piece) = self.piece_at_square(m.from()) {
            return piece.kind().value();
        }

        0
    }

    fn move_value(&self, m: Move) -> i32 {
        if let OptionPiece::Some(piece) = self.piece_at_square(m.capture_square()) {
            let mut value = piece.kind().value();
            if let Some(promotion_piece) = m.promoted_piece() {
                value += promotion_piece.value() - Piece::Pawn.value();
            }

            return value;
        }

        0
    }

    fn least_valuable_attacker(&self, attackers: BitBoard) -> Piece {
        if !(attackers & self.state.pieces[Piece::Pawn]).is_empty() {
            return Piece::Pawn;
        }

        if !(attackers & self.state.pieces[Piece::Knight]).is_empty() {
            return Piece::Knight;
        }

        if !(attackers & self.state.pieces[Piece::Bishop]).is_empty() {
            return Piece::Bishop;
        }

        if !(attackers & self.state.pieces[Piece::Rook]).is_empty() {
            return Piece::Rook;
        }

        if !(attackers & self.state.pieces[Piece::Queen]).is_empty() {
            return Piece::Queen;
        }

        if !(attackers & self.state.pieces[Piece::King]).is_empty() {
            return Piece::King;
        }

        unreachable!("No attackers");
    }

    pub fn attackers_to(&self, square: Square, occupancies: BitBoard) -> BitBoard {
        let diagonals = self.state.pieces[Piece::Bishop] | self.state.pieces[Piece::Queen];
        let orthogonals = self.state.pieces[Piece::Rook] | self.state.pieces[Piece::Queen];

        (bishop_attacks(square, occupancies) & diagonals)
            | (rook_attacks(square, occupancies) & orthogonals)
            | (pawn_attacks(square, Side::White) & self.piece_bb(Side::Black, Piece::Pawn))
            | (pawn_attacks(square, Side::Black) & self.piece_bb(Side::White, Piece::Pawn))
            | (knight_attacks(square) & self.state.pieces[Piece::Knight])
            | (king_attacks(square) & self.state.pieces[Piece::King])
    }
}

pub fn value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 88,
        Piece::Knight => 429,
        Piece::Bishop => 454,
        Piece::Rook => 654,
        Piece::Queen => 1293,
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
            board: Board::from_fen("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("e2e5").unwrap();
        assert!(!data.board.see(m, -150));

        let data = SearchData {
            board: Board::from_fen("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1").unwrap(),
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
            board: Board::from_fen("1k1r3q/1pp4p/pn3b2/4p3/P7/3N2P1/1PP1R1BP/2K1Q3 w - - 1 2").unwrap(),
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
            board: Board::from_fen("r1bqk2r/ppp1p1pp/3p2n1/3P4/4PN2/5b2/PPPP2Pp/RNBQK1R1 b Qkq - 0 1").unwrap(),
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
    }
}
