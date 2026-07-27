use super::Board;
use crate::types::{
    CASTLING_ROOK_SQAURES, Castling, Piece, Side, Square,
    moves::{Move, MoveKind},
};

impl Board {
    pub fn make_move(&mut self, m: Move) {
        let from = m.get_from();
        let to = m.get_to();
        let kind = m.get_kind();
        let (side, piece) = self.get_piece_at_square(from).unwrap();

        let king_rook_square = CASTLING_ROOK_SQAURES[side][Castling::KING_SIDE];
        let queen_rook_square = CASTLING_ROOK_SQAURES[side][Castling::QUEEN_SIDE];
        let opp_king_rook_square = CASTLING_ROOK_SQAURES[side.other()][Castling::KING_SIDE];
        let opp_queen_rook_square = CASTLING_ROOK_SQAURES[side.other()][Castling::QUEEN_SIDE];

        self.copy_state();
        self.state.plies_from_null += 1;
        self.state.keys.toggle_castling(self.state.castling_rights);

        if let Some(square) = self.state.enpassant {
            self.state.keys.toggle_en_passant(square);
            self.state.enpassant = None;
        }

        if let Piece::King = piece {
            self.state.castling_rights.clear_king_side(side);
            self.state.castling_rights.clear_queen_side(side);
        }

        self.state.side_to_move = self.state.side_to_move.other();
        self.state.keys.toggle_side();

        if let Piece::Rook = piece {
            if from == king_rook_square && self.state.castling_rights.can_king_side(side) {
                self.state.castling_rights.clear_king_side(side);
            }

            if from == queen_rook_square && self.state.castling_rights.can_queen_side(side) {
                self.state.castling_rights.clear_queen_side(side);
            }
        }

        if let Some(castle_kind) = m.castle_kind() {
            let offset = [1, -1];
            self.remove_piece(side, Piece::Rook, CASTLING_ROOK_SQAURES[side][castle_kind]);
            self.state.castling_rights.clear_king_side(side);
            self.state.castling_rights.clear_queen_side(side);
            self.place_piece(side, Piece::Rook, from.shift(offset[castle_kind]).unwrap());
        }

        if kind == MoveKind::DoublePawn {
            self.state.enpassant = Some(Square::from(to as usize ^ 8));
            self.state
                .keys
                .toggle_en_passant(Square::from(to as usize ^ 8));
        }

        if let Some((other_side, captured_piece)) = self.get_piece_at_square(m.get_capture_square())
        {
            self.remove_piece(other_side, captured_piece, m.get_capture_square());
            if captured_piece == Piece::Rook {
                if to == opp_king_rook_square {
                    self.state.castling_rights.clear_king_side(other_side);
                }
                if to == opp_queen_rook_square {
                    self.state.castling_rights.clear_queen_side(other_side);
                }
            }
        }

        self.remove_piece(side, piece, from);
        if let Some(promotion_piece) = m.get_promoted_piece() {
            self.place_piece(side, promotion_piece, to);
        } else {
            self.place_piece(side, piece, to);
        }

        //Irreversible Move
        if kind.is_capture() || piece == Piece::Pawn {
            self.state.half_move_clock = 0
        } else {
            self.state.half_move_clock += 1
        }

        if self.state.side_to_move == Side::Black {
            self.state.full_move += 1
        }

        self.state.keys.toggle_castling(self.state.castling_rights);
        self.update_all_threats();
        self.update_en_passant();
        self.game_history.push(self.state.keys.full);
    }

    pub fn update_en_passant(&mut self) {
        if let Some(enpassant) = self.state.enpassant {
            let stm = self.state.side_to_move;
            let king_square = self.get_king_square(stm);
            let pawn_square = Square::from(enpassant as usize ^ 8);

            //Update occupancy as if enpassant pawn was taken for each possible ep taker
            let occupancies = self.get_all_occupancy() ^ enpassant.to_bb() ^ pawn_square.to_bb();
            let possible_takers =
                self.get_pawn_attacks(enpassant, stm.other()) & self.get_piece_bb(stm, Piece::Pawn);

            debug_assert!(possible_takers.count_bits() <= 2);

            for taker in possible_takers.iter() {
                let new_occ = occupancies ^ taker.to_bb();
                let bishop_queens = self.get_piece_bb(stm.other(), Piece::Bishop)
                    | self.get_piece_bb(stm.other(), Piece::Queen);
                let bishop_queen_checkers =
                    self.get_bishop_attacks(king_square, new_occ) & bishop_queens;

                let rook_queens = self.get_piece_bb(stm.other(), Piece::Rook)
                    | self.get_piece_bb(stm.other(), Piece::Queen);
                let rook_queen_checkers = self.get_rook_attacks(king_square, new_occ) & rook_queens;
                let checkers = bishop_queen_checkers | rook_queen_checkers;

                if checkers.is_empty() {
                    //En Passant is allowed
                    return;
                }
            }

            //Toggle en passant off
            self.state.keys.toggle_en_passant(enpassant);
            self.state.enpassant = None;
        }
    }

    pub fn unmake_move(&mut self) {
        if let Some(prev_state) = self.state_stack.pop() {
            self.state = prev_state;
        }

        self.game_history.pop();
    }

    pub fn copy_state(&mut self) {
        self.state_stack.push(self.state.clone());
    }

    pub fn king_in_check(&self) -> bool {
        !self.state.checkers.is_empty()
    }

    pub fn make_null_move(&mut self) {
        self.copy_state();
        self.state.side_to_move = self.state.side_to_move.other();
        self.state.keys.toggle_side();
        self.state.plies_from_null = 0;
        self.state.half_move_clock += 1;

        if let Some(square) = self.state.enpassant {
            self.state.keys.toggle_en_passant(square);
            self.state.enpassant = None;
        }

        if self.state.side_to_move == Side::White {
            self.state.full_move += 1
        }

        self.game_history.push(self.state.keys.full);
        self.update_all_threats();
    }
}

#[cfg(test)]
mod tests {
    use crate::board::Board;
    use crate::search::data::SearchData;
    use crate::types::{
        Piece, Side, Square,
        moves::{Move, MoveKind},
    };

    #[test]
    fn test_make_move() {
        let mut board =
            Board::from_fen("1K6/3pp1P1/4R3/3k3p/Ppn5/4b3/1PP1P1p1/7B b - a3 0 1").unwrap();
        println!("{board}");

        let m = Move::new(Square::B4, Square::A3, MoveKind::EnPassant);
        board.make_move(m);
        println!("{board}");
        assert_eq!(
            board.get_piece_at_square(Square::A3).unwrap(),
            (Side::Black, Piece::Pawn)
        );
        assert!(board.get_piece_at_square(Square::A4).is_none());

        board.unmake_move();
        println!("{board}");
        assert_eq!(
            board.get_piece_at_square(Square::B4).unwrap(),
            (Side::Black, Piece::Pawn)
        );

        let m = Move::new(Square::C4, Square::B2, MoveKind::Capture);
        board.make_move(m);
        println!("{board}");
        assert_eq!(
            board.get_piece_at_square(Square::B2).unwrap(),
            (Side::Black, Piece::Knight)
        );

        let mut board = Board::from_fen(
            "r3k2r/pppqn2p/n1bp2pb/1N2p3/2B5/1QP1PN2/PP1B1PPP/R3K2R w KQkq - 10 12",
        )
        .unwrap();
        println!("{board}");

        let m = Move::new(Square::E1, Square::G1, MoveKind::KingCastle);
        board.make_move(m);
        println!("{board}");
        assert_eq!(
            board.get_piece_at_square(Square::G1).unwrap(),
            (Side::White, Piece::King)
        );
        assert_eq!(
            board.get_piece_at_square(Square::F1).unwrap(),
            (Side::White, Piece::Rook)
        );
        assert!(board.get_piece_at_square(Square::E1).is_none());

        let m = Move::new(Square::E8, Square::C8, MoveKind::QueenCastle);
        board.make_move(m);
        println!("{board}");
        assert_eq!(
            board.get_piece_at_square(Square::C8).unwrap(),
            (Side::Black, Piece::King)
        );
        assert_eq!(
            board.get_piece_at_square(Square::D8).unwrap(),
            (Side::Black, Piece::Rook)
        );
        assert!(board.get_piece_at_square(Square::E8).is_none());

        let mut board =
            Board::from_fen("2kr3r/pppqn2p/n1b3pb/1N2p3/2B5/1QP4N/PP2pPPP/R1B2R1K b - - 1 17")
                .unwrap();
        println!("{board}");

        let m = Move::new(Square::E2, Square::E1, MoveKind::BPromotion);
        board.make_move(m);
        println!("{board}");
        assert_eq!(
            board.get_piece_at_square(Square::E1).unwrap(),
            (Side::Black, Piece::Bishop)
        );
        assert!(board.get_piece_at_square(Square::E2).is_none());

        board.unmake_move();
        let m = Move::new(Square::E2, Square::F1, MoveKind::QPromCapture);
        board.make_move(m);
        println!("{board}");
        assert_eq!(
            board.get_piece_at_square(Square::F1).unwrap(),
            (Side::Black, Piece::Queen)
        );
        assert!(board.get_piece_at_square(Square::E2).is_none());

        let mut board =
            Board::from_fen("r3k2N/p1ppqpb1/bn2pn2/3P4/1p2P3/2N2Q2/PPPBBPpP/R3K2R b KQq - 0 2")
                .unwrap();
        println!("{board}");

        let m = Move::new(Square::G2, Square::H1, MoveKind::NPromCapture);
        board.make_move(m);
        assert!(!board.state.castling_rights.can_king_side(Side::White));
        println!("{board}");
    }

    #[test]
    fn test_null_move() {
        let mut board =
            Board::from_fen("2kr3r/pppqn2p/n1b3pb/1N2p3/2B5/1QP4N/PP2pPPP/R1B2R1K b - - 1 17")
                .unwrap();
        println!("{board}");

        let original =
            Board::from_fen("2kr3r/pppqn2p/n1b3pb/1N2p3/2B5/1QP4N/PP2pPPP/R1B2R1K b - - 1 17")
                .unwrap();

        board.make_null_move();
        assert_eq!(board.state.side_to_move, Side::White);
        println!("{board}");
        board.unmake_move();

        assert_eq!(board, original);
    }

    #[test]
    fn test_update_ep() {
        let _ = SearchData {
            board: Board::from_fen("8/2p5/3p4/KP5r/1R3pPk/8/4P3/8 b - g3 0 1").unwrap(),
            ..Default::default()
        };
    }
}
