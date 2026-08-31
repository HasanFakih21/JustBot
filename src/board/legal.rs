use crate::{
    attacks::{BETWEEN, RAYS},
    board::Board,
    lookup::{attacks, bishop_attacks, king_attacks, pawn_attacks, rook_attacks},
    types::{
        BitBoard, Castling, KING_TO, Move, MoveKind, NORTH, OptionPiece, Piece, RANK_1, RANK_8,
        ROOK_TO, SOUTH, Side,
    },
};

impl Board {
    pub fn is_legal(&self, m: Move) -> bool {
        let stm = self.state.side_to_move;
        let from = m.from();
        let to = m.to();
        let king_square = self.king_square(stm);

        let OptionPiece::Some(piece) = self.piece_at_square(from) else {
            return false;
        };

        if piece.side() != stm {
            return false;
        }

        let moving_piece = piece.kind();
        // Verify King Moves
        if moving_piece == Piece::King {
            if let Some(dir) = m.castle_direction() {
                let king_to = KING_TO[stm][dir];
                let rook_to = ROOK_TO[stm][dir];
                let rook_square = self.castling_rooks[stm][dir];

                // Needs to be empty
                let mut between = BETWEEN[king_square][king_to]
                    | BETWEEN[rook_square][rook_to]
                    | rook_to.to_bb()
                    | king_to.to_bb();
                between &= !king_square.to_bb();
                between &= !rook_square.to_bb();

                // Can't be under attack
                let king_path = BETWEEN[king_square][to] | king_square.to_bb() | to.to_bb();

                return self.state.castling_rights.can(Castling::KINDS[stm][dir])
                    && (between & self.all_occupancy()).is_empty()
                    && (king_path & self.state.threats).is_empty()
                    && !self.state.pinned[stm].contains(rook_square);
            }

            return matches!(m.kind(), MoveKind::Capture | MoveKind::QuietMove)
                && !self.state.occupancies[stm].contains(to)
                && m.is_capture() == self.state.occupancies[stm.other()].contains(to)
                && (king_attacks(from) & !self.state.threats).contains(to);
        }

        if self.state.occupancies[stm].contains(to) // If to square has piece of the same side
            || self.state.pinned[stm].contains(from) && !RAYS[from][king_square].contains(to) // If piece is pinned and the to square isn't on the same ray as the king
            || self.king_in_check()
                && (self.state.checkers.count_bits() > 1 // If there's multiple checkers then the king has to move 
                // If it's a check and it also doesn't contain a move that's between the king and checking piece or a capture of the checking piece
                || ((m.kind() != MoveKind::EnPassant) && !(self.state.checkers | BETWEEN[king_square][self.state.checkers.least_sig_bit().unwrap()]).contains(to)))
        {
            return false;
        }

        // Verify pawn moves
        if moving_piece == Piece::Pawn {
            if m.is_en_passant() {
                let Some(ep_square) = self.state.enpassant else {
                    return false;
                };

                let occupancies =
                    self.all_occupancy() ^ from.to_bb() ^ to.to_bb() ^ (to ^ 8).to_bb();
                let bishop_queens = self.piece_bb(stm.other(), Piece::Bishop)
                    | self.piece_bb(stm.other(), Piece::Queen);
                let rook_queens = self.piece_bb(stm.other(), Piece::Rook)
                    | self.piece_bb(stm.other(), Piece::Queen);
                let diagonal = bishop_attacks(king_square, occupancies) & bishop_queens;
                let orthogonal = rook_attacks(king_square, occupancies) & rook_queens;
                return to == ep_square
                    && pawn_attacks(from, stm).contains(to)
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
                return pawn_attacks(from, stm).contains(to)
                    && self.state.occupancies[stm.other()].contains(to);
            }

            let offset = match stm {
                Side::White => NORTH,
                Side::Black => SOUTH,
            };

            if m.kind() == MoveKind::DoublePawn {
                let home_rank = match stm {
                    Side::White => 1,
                    Side::Black => 6,
                };

                return from.to_rank() == home_rank
                    && from.shift(2 * offset) == to
                    && !self.all_occupancy().contains(from.shift(offset))
                    && !self.all_occupancy().contains(to);
            }

            return !m.is_castling()
                && from.shift(offset) == to
                && !self.all_occupancy().contains(to);
        }

        matches!(m.kind(), MoveKind::Capture | MoveKind::QuietMove)
            && m.is_capture() == self.state.occupancies[stm.other()].contains(to)
            && attacks(stm, from, moving_piece, self.all_occupancy()).contains(to)
    }
}
