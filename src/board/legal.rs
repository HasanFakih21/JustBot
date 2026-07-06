use crate::{
    attacks::{BETWEEN, RAYS},
    board::Board,
    types::{
        B_FILE, BitBoard, Move, MoveKind, NORTH, Piece, RANK_1, RANK_8, SOUTH, Side, WK_SIDE,
        WQ_SIDE,
    },
};

impl Board {
    pub fn is_legal(&self, m: Move) -> bool {
        let stm = self.state.side_to_move;
        let from = m.get_from();
        let to = m.get_to();
        let king_square = self.get_king_square(stm);

        let Some((moving_piece_side, moving_piece)) = self.get_piece_at_square(from) else {
            return false;
        };

        if moving_piece_side != stm {
            return false;
        }

        //Verify King Moves
        if moving_piece == Piece::King {
            if m.get_kind() == MoveKind::KingCastle {
                let king_side_path = match stm {
                    Side::White => BitBoard(WK_SIDE),
                    Side::Black => BitBoard(WK_SIDE).shift(NORTH * 7),
                };

                let path =
                    (king_side_path | self.get_piece_bb(stm, Piece::King)) & self.state.threats;
                return self.state.castling_rights.can_king_side(stm)
                    && (king_side_path & self.get_all_occupancy()).is_empty()
                    && path.is_empty();
            }

            if m.get_kind() == MoveKind::QueenCastle {
                let queen_side_path = match stm {
                    Side::White => BitBoard(WQ_SIDE),
                    Side::Black => BitBoard(WQ_SIDE).shift(NORTH * 7),
                };

                let need_to_be_safe = (queen_side_path ^ BitBoard(B_FILE)) & queen_side_path;
                let path =
                    (need_to_be_safe | self.get_piece_bb(stm, Piece::King)) & self.state.threats;
                return self.state.castling_rights.can_queen_side(stm)
                    && (queen_side_path & self.get_all_occupancy()).is_empty()
                    && path.is_empty();
            }

            return matches!(m.get_kind(), MoveKind::Capture | MoveKind::QuietMove)
                && !self.state.occupancies[stm as usize].contains(to)
                && m.is_capture() == self.state.occupancies[stm.other() as usize].contains(to)
                && (self.get_king_attacks(from) & !self.state.threats).contains(to);
        }

        if self.state.occupancies[stm as usize].contains(to) //If to square has piece of the same side
            || self.state.pinned[stm as usize].contains(from) && !RAYS[from as usize][king_square as usize].contains(to) //If piece is pinned and the to square isn't on the same ray as the king
            || self.king_in_check()
                && (self.state.checkers.count_bits() > 1 //If there's multiple checkers then the king has to move 
                || ((m.get_kind() != MoveKind::EnPassant) && !(self.state.checkers | BETWEEN[king_square as usize][unsafe { self.state.checkers.least_sig_bit().unwrap_unchecked() as usize }]).contains(to)))
        //If it's a check and it also doesn't contain a move that's between the king and checking piece or a capture of the checking piece
        {
            return false;
        }

        //Verify pawn moves
        if moving_piece == Piece::Pawn {
            if m.is_en_passant() {
                let Some(ep_square) = self.state.enpassant else {
                    return false;
                };

                let occupancies =
                    self.get_all_occupancy() ^ from.to_bb() ^ to.to_bb() ^ (to ^ 8).to_bb();
                let bishop_queens = self.get_piece_bb(stm.other(), Piece::Bishop)
                    | self.get_piece_bb(stm.other(), Piece::Queen);
                let rook_queens = self.get_piece_bb(stm.other(), Piece::Rook)
                    | self.get_piece_bb(stm.other(), Piece::Queen);
                let diagonal = self.get_bishop_attacks(king_square, occupancies) & bishop_queens;
                let orthogonal = self.get_rook_attacks(king_square, occupancies) & rook_queens;
                return to == ep_square
                    && self.get_pawn_attacks(from, stm).contains(to)
                    && (orthogonal | diagonal).is_empty();
            }

            if m.is_promotion() {
                let promotion_rank = match stm {
                    Side::White => BitBoard(RANK_8),
                    Side::Black => BitBoard(RANK_1),
                };

                if !promotion_rank.contains(to) {
                    return false;
                }
            }

            if m.is_capture() {
                return self.get_pawn_attacks(from, stm).contains(to)
                    && self.state.occupancies[stm.other() as usize].contains(to);
            }

            let offset = match stm {
                Side::White => NORTH,
                Side::Black => SOUTH,
            };

            let next_square = from.shift(offset);

            if m.get_kind() == MoveKind::DoublePawn {
                let next_two_square = from.shift(2 * offset); 
                let home_rank = match stm {
                    Side::White => 1,
                    Side::Black => 6,
                };

                return from.to_rank() == home_rank
                    && !self.get_all_occupancy().contains(next_square)
                    && !self.get_all_occupancy().contains(next_two_square);
            }

            return !m.is_castling() && next_square == to && !self.get_all_occupancy().contains(to);
        }

        matches!(m.get_kind(), MoveKind::Capture | MoveKind::QuietMove)
            && m.is_capture() == self.state.occupancies[stm.other() as usize].contains(to)
            && self.get_piece_attack(stm, from, moving_piece).contains(to)
    }
}
