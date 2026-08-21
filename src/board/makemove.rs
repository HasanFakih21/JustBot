use super::Board;
use crate::types::{
    Castling, OptionPiece, Piece, ROOK_TO, Side, Square,
    moves::{Move, MoveKind},
};

impl Board {
    pub fn make_move(&mut self, m: Move) {
        let from = m.get_from();
        let to = m.get_to();
        let kind = m.get_kind();
        let side_piece = self.get_piece_at_square(from).unwrap();
        let side = side_piece.side();
        let piece = side_piece.kind();

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

        if let Piece::Rook = piece {
            if self.state.castling_rights.can_king_side(side)
                && from == self.castling_rooks[side][Castling::KING_SIDE]
            {
                self.state.castling_rights.clear_king_side(side);
            } else if self.state.castling_rights.can_queen_side(side)
                && from == self.castling_rooks[side][Castling::QUEEN_SIDE]
            {
                self.state.castling_rights.clear_queen_side(side);
            }
        }

        if kind == MoveKind::DoublePawn {
            self.state.enpassant = Some(Square::from(to as usize ^ 8));
            self.state
                .keys
                .toggle_en_passant(Square::from(to as usize ^ 8));
        }

        if let Some(castle_kind) = m.castle_direction() {
            self.remove_piece(side, piece, from);
            self.remove_piece(side, Piece::Rook, self.castling_rooks[side][castle_kind]);
            self.state.castling_rights.clear_king_side(side);
            self.state.castling_rights.clear_queen_side(side);
            self.place_piece(side, Piece::Rook, ROOK_TO[side][castle_kind]);
        } else if let OptionPiece::Some(sided_piece) =
            self.get_piece_at_square(m.get_capture_square())
        {
            let other_side = sided_piece.side();
            let captured_piece = sided_piece.kind();
            self.remove_piece(side, piece, from);
            self.remove_piece(other_side, captured_piece, m.get_capture_square());
            if captured_piece == Piece::Rook {
                if to == self.castling_rooks[other_side][Castling::KING_SIDE] {
                    self.state.castling_rights.clear_king_side(other_side);
                } else if to == self.castling_rooks[other_side][Castling::QUEEN_SIDE] {
                    self.state.castling_rights.clear_queen_side(other_side);
                }
            }
        } else {
            self.remove_piece(side, piece, from);
        }

        if let Some(promotion_piece) = m.get_promoted_piece() {
            self.place_piece(side, promotion_piece, to);
        } else {
            self.place_piece(side, piece, to);
        }

        // Irreversible Move
        if kind.is_capture() || piece == Piece::Pawn {
            self.state.half_move_clock = 0
        } else {
            self.state.half_move_clock += 1
        }

        if self.state.side_to_move == Side::Black {
            self.state.full_move += 1
        }

        self.state.side_to_move = self.state.side_to_move.other();
        self.state.keys.toggle_side();
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

            // Update occupancy as if enpassant pawn was taken for each possible ep taker
            let occupancies = self.get_all_occupancy() ^ enpassant.to_bb() ^ pawn_square.to_bb();
            let possible_takers =
                self.get_pawn_attacks(enpassant, stm.other()) & self.get_piece_bb(stm, Piece::Pawn);

            debug_assert!(possible_takers.count_bits() <= 2);

            for taker in possible_takers {
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
                    // En Passant is allowed
                    return;
                }
            }

            // Toggle en passant off
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
        self.state.plies_from_null = 0;

        if let Some(square) = self.state.enpassant {
            self.state.keys.toggle_en_passant(square);
            self.state.enpassant = None;
        }

        if self.state.side_to_move == Side::Black {
            self.state.full_move += 1
        }

        self.state.side_to_move = self.state.side_to_move.other();
        self.state.keys.toggle_side();
        self.game_history.push(self.state.keys.full);
        self.update_all_threats();
    }
}
