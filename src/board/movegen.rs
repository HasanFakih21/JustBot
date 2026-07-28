use crate::attacks::{BETWEEN, DIAGONALS, RAYS};
use crate::board::Board;
use crate::types::*;

#[derive(Debug, Clone, Copy)]
pub enum MoveGenKind {
    All,
    Quiet,
    Noisy,
}

impl MoveGenKind {
    pub fn is_quiet(&self) -> bool {
        matches!(self, Self::Quiet | Self::All)
    }

    pub fn is_noisy(&self) -> bool {
        matches!(self, Self::Noisy | Self::All)
    }
}

impl Board {
    pub fn gen_pawn_moves(&self, move_list: &mut MoveList, kind: MoveGenKind) {
        let stm = self.state.side_to_move;
        let offset = match stm {
            Side::White => NORTH,
            Side::Black => SOUTH,
        };

        let (left, right) = match stm {
            Side::White => (offset + WEST, offset + EAST),
            Side::Black => (offset + EAST, offset + WEST),
        };

        let promotion_rank = match stm {
            Side::White => BitBoard(RANK_8),
            Side::Black => BitBoard(RANK_1),
        };

        let third_rank = match stm {
            Side::White => BitBoard(RANK_4).shift(SOUTH),
            Side::Black => BitBoard(RANK_5).shift(NORTH),
        };

        let pinned = self.state.pinned[stm as usize];
        let king_square = self.get_king_square(stm);
        let pawns = self.get_piece_bb(stm, Piece::Pawn);
        let occupied = self.get_all_occupancy();

        let target = if self.king_in_check() {
            debug_assert!(self.state.checkers.count_bits() == 1);
            // Only moves that can block the check
            let checking_piece_square = self.state.checkers.least_sig_bit().unwrap();
            BETWEEN[king_square as usize][checking_piece_square as usize] | self.state.checkers
        } else {
            !BitBoard(0)
        };

        let pushes = (pawns & (!pinned | to_file_bb(king_square))).shift(offset) & !occupied;
        let promotions = pushes & promotion_rank & target;

        if kind.is_quiet() {
            let single_pushes = pushes ^ promotions;
            let double_pushes = (single_pushes & third_rank).shift(offset) & !occupied;

            // Single Push
            for to in (single_pushes & target).iter() {
                move_list.push(Move::new(
                    to.shift(-offset).unwrap(),
                    to,
                    MoveKind::QuietMove,
                ));
            }

            // Double Push
            for to in (double_pushes & target).iter() {
                move_list.push(Move::new(
                    to.shift(-offset * 2).unwrap(),
                    to,
                    MoveKind::DoublePawn,
                ));
            }
        }

        if kind.is_noisy() {
            // Normal Promotions
            for to in promotions.iter() {
                move_list.push(Move::new(
                    to.shift(-offset).unwrap(),
                    to,
                    MoveKind::QPromotion,
                ));
                move_list.push(Move::new(
                    to.shift(-offset).unwrap(),
                    to,
                    MoveKind::RPromotion,
                ));
                move_list.push(Move::new(
                    to.shift(-offset).unwrap(),
                    to,
                    MoveKind::BPromotion,
                ));
                move_list.push(Move::new(
                    to.shift(-offset).unwrap(),
                    to,
                    MoveKind::NPromotion,
                ));
            }

            // Captures
            let target = target & self.state.occupancies[stm.other() as usize];

            let left_pawns = (pawns & (!pinned | DIAGONALS[1][king_square as usize]))
                & match stm {
                    Side::White => !A,
                    Side::Black => !H,
                };
            let right_pawns = (pawns & (!pinned | DIAGONALS[0][king_square as usize]))
                & match stm {
                    Side::White => !H,
                    Side::Black => !A,
                };

            let left_captures = left_pawns.shift(left) & target;
            let left_promos = left_captures & promotion_rank;
            move_list.push_promotion_captures_setwise(left, left_promos);
            move_list.push_pawn_moves_setwise(left, left_captures ^ left_promos, MoveKind::Capture);

            let right_captures = right_pawns.shift(right) & target;
            let right_promos = right_captures & promotion_rank;
            move_list.push_promotion_captures_setwise(right, right_promos);
            move_list.push_pawn_moves_setwise(
                right,
                right_captures ^ right_promos,
                MoveKind::Capture,
            );
            if let Some(en_passant) = self.state.enpassant {
                if left_pawns.contains(en_passant.shift(-left).unwrap()) {
                    let from = en_passant.shift(-left).unwrap();
                    move_list.push(Move::new(from, en_passant, MoveKind::EnPassant));
                }

                if right_pawns.contains(en_passant.shift(-right).unwrap()) {
                    let from = en_passant.shift(-right).unwrap();
                    move_list.push(Move::new(from, en_passant, MoveKind::EnPassant));
                }
            }
        }
    }

    pub fn gen_castling_moves(&self, move_list: &mut MoveList) {
        let side = self.state.side_to_move;
        let king = self.get_piece_bb(side, Piece::King);
        let occupancies = self.get_all_occupancy();
        let mut king_side_occ = BitBoard(WK_SIDE);
        let mut queen_side_occ = BitBoard(WQ_SIDE);
        if side == Side::Black {
            king_side_occ = king_side_occ.shift(NORTH * 7);
            queen_side_occ = queen_side_occ.shift(NORTH * 7);
        }
        let need_to_be_safe = (queen_side_occ ^ BitBoard(B_FILE)) & queen_side_occ;

        if self.state.castling_rights.can_king_side(side)
            && ((king_side_occ & occupancies).0 == 0)
            && !(king_side_occ | king).iter().any(|e| self.is_attacked(e))
        {
            let target = match side {
                Side::White => Castling::WhiteKing.king_landing_square(),
                Side::Black => Castling::BlackKing.king_landing_square(),
            };
            move_list.push(Move::new(
                king.least_sig_bit().unwrap(),
                target,
                MoveKind::KingCastle,
            ));
        }

        if self.state.castling_rights.can_queen_side(side)
            && ((queen_side_occ & occupancies).0 == 0)
            && !(need_to_be_safe | king).iter().any(|e| self.is_attacked(e))
        {
            let target = match side {
                Side::White => Castling::WhiteQueen.king_landing_square(),
                Side::Black => Castling::BlackQueen.king_landing_square(),
            };
            move_list.push(Move::new(
                king.least_sig_bit().unwrap(),
                target,
                MoveKind::QueenCastle,
            ));
        }
    }

    pub fn gen_knight_moves(&self, move_list: &mut MoveList, kind: MoveGenKind) {
        let stm = self.state.side_to_move;
        let king_square = self.get_king_square(stm);
        let occupied = self.get_all_occupancy();
        let pinned = self.state.pinned[stm as usize];

        let target = if self.king_in_check() {
            debug_assert!(self.state.checkers.count_bits() == 1);
            // Only moves that can block the check
            let checking_piece_square = self.state.checkers.least_sig_bit().unwrap();
            BETWEEN[king_square as usize][checking_piece_square as usize] | self.state.checkers
        } else {
            !BitBoard(0)
        };

        let knights = self.get_piece_bb(stm, Piece::Knight);
        if kind.is_noisy() {
            let target = target & self.state.occupancies[stm.other() as usize];
            for from in (knights & !pinned).iter() {
                move_list.push_setwise(
                    from,
                    self.get_knight_attacks(from) & target,
                    MoveKind::Capture,
                );
            }
        }

        if kind.is_quiet() {
            let target = target & !occupied;
            for from in (knights & !pinned).iter() {
                move_list.push_setwise(
                    from,
                    self.get_knight_attacks(from) & target,
                    MoveKind::QuietMove,
                );
            }
        }
    }

    pub fn gen_sliding_moves<F: Fn(Square) -> BitBoard>(
        &self,
        move_list: &mut MoveList,
        kind: MoveKind,
        pieces: BitBoard,
        attacks: F,
        target: BitBoard,
        pinned: BitBoard,
    ) {
        for from in (pieces & !pinned).iter() {
            move_list.push_setwise(from, attacks(from) & target, kind);
        }

        let king_square = self.get_king_square(self.state.side_to_move);
        for from in (pieces & pinned).iter() {
            move_list.push_setwise(
                from,
                attacks(from) & target & RAYS[from as usize][king_square as usize],
                kind,
            );
        }
    }

    pub fn gen_bishop_moves(&self, move_list: &mut MoveList, kind: MoveGenKind) {
        let stm = self.state.side_to_move;
        let king_square = self.get_king_square(stm);
        let occupied = self.get_all_occupancy();
        let pinned = self.state.pinned[stm as usize];

        let target = if self.king_in_check() {
            debug_assert!(self.state.checkers.count_bits() == 1);
            // Only moves that can block the check
            let checking_piece_square = self.state.checkers.least_sig_bit().unwrap();
            BETWEEN[king_square as usize][checking_piece_square as usize] | self.state.checkers
        } else {
            !BitBoard(0)
        };

        let bishops = self.get_piece_bb(stm, Piece::Bishop);
        let attacks = |square| self.get_bishop_attacks(square, occupied);

        if kind.is_noisy() {
            let target = target & self.state.occupancies[stm.other() as usize];
            self.gen_sliding_moves(
                move_list,
                MoveKind::Capture,
                bishops,
                attacks,
                target,
                pinned,
            );
        }

        if kind.is_quiet() {
            let target = target & !occupied;
            self.gen_sliding_moves(
                move_list,
                MoveKind::QuietMove,
                bishops,
                attacks,
                target,
                pinned,
            );
        }
    }

    pub fn gen_rook_moves(&self, move_list: &mut MoveList, kind: MoveGenKind) {
        let stm = self.state.side_to_move;
        let king_square = self.get_king_square(stm);
        let occupied = self.get_all_occupancy();
        let pinned = self.state.pinned[stm as usize];

        let target = if self.king_in_check() {
            debug_assert!(self.state.checkers.count_bits() == 1);
            // Only moves that can block the check
            let checking_piece_square = self.state.checkers.least_sig_bit().unwrap();
            BETWEEN[king_square as usize][checking_piece_square as usize] | self.state.checkers
        } else {
            !BitBoard(0)
        };

        let rooks = self.get_piece_bb(stm, Piece::Rook);
        let attacks = |square| self.get_rook_attacks(square, occupied);

        if kind.is_noisy() {
            let target = target & self.state.occupancies[stm.other() as usize];
            self.gen_sliding_moves(move_list, MoveKind::Capture, rooks, attacks, target, pinned);
        }

        if kind.is_quiet() {
            let target = target & !occupied;
            self.gen_sliding_moves(
                move_list,
                MoveKind::QuietMove,
                rooks,
                attacks,
                target,
                pinned,
            );
        }
    }

    pub fn gen_queen_moves(&self, move_list: &mut MoveList, kind: MoveGenKind) {
        let stm = self.state.side_to_move;
        let king_square = self.get_king_square(stm);
        let occupied = self.get_all_occupancy();
        let pinned = self.state.pinned[stm as usize];

        let target = if self.king_in_check() {
            debug_assert!(self.state.checkers.count_bits() == 1);
            // Only moves that can block the check
            let checking_piece_square = self.state.checkers.least_sig_bit().unwrap();
            BETWEEN[king_square as usize][checking_piece_square as usize] | self.state.checkers
        } else {
            !BitBoard(0)
        };

        let queens = self.get_piece_bb(stm, Piece::Queen);
        let attacks = |square| self.get_queen_attacks(square, occupied);

        if kind.is_noisy() {
            let target = target & self.state.occupancies[stm.other() as usize];
            self.gen_sliding_moves(
                move_list,
                MoveKind::Capture,
                queens,
                attacks,
                target,
                pinned,
            );
        }

        if kind.is_quiet() {
            let target = target & !occupied;
            self.gen_sliding_moves(
                move_list,
                MoveKind::QuietMove,
                queens,
                attacks,
                target,
                pinned,
            );
        }
    }

    pub fn gen_king_moves(&self, move_list: &mut MoveList, kind: MoveGenKind) {
        let stm = self.state.side_to_move;
        let occupancies = self.get_all_occupancy();
        let king_square = self.get_king_square(stm);

        let attacks = self.get_king_attacks(king_square);
        let mut targets = BitBoard(0);
        if kind.is_quiet() {
            targets |= !occupancies & attacks & !self.state.threats;
            for target in targets.iter() {
                move_list.push(Move::new(king_square, target, MoveKind::QuietMove));
            }

            targets = BitBoard(0);
        }

        if kind.is_noisy() {
            targets |= self.state.occupancies[stm.other() as usize] & attacks & !self.state.threats;
            for target in targets.iter() {
                move_list.push(Move::new(king_square, target, MoveKind::Capture));
            }
        }
    }

    pub fn generate_moves(&self, kind: MoveGenKind) -> MoveList {
        let mut move_list = MoveList::new();
        self.gen_king_moves(&mut move_list, kind);
        if self.state.checkers.count_bits() > 1 {
            return move_list;
        }

        self.gen_pawn_moves(&mut move_list, kind);
        self.gen_knight_moves(&mut move_list, kind);
        self.gen_bishop_moves(&mut move_list, kind);
        self.gen_rook_moves(&mut move_list, kind);
        self.gen_queen_moves(&mut move_list, kind);

        if kind.is_quiet() {
            self.gen_castling_moves(&mut move_list)
        }

        move_list
    }

    pub fn append_moves(&self, kind: MoveGenKind, move_list: &mut MoveList) {
        self.gen_king_moves(move_list, kind);
        if self.state.checkers.count_bits() > 1 {
            return;
        }

        self.gen_pawn_moves(move_list, kind);
        self.gen_knight_moves(move_list, kind);
        self.gen_bishop_moves(move_list, kind);
        self.gen_rook_moves(move_list, kind);
        self.gen_queen_moves(move_list, kind);

        if kind.is_quiet() {
            self.gen_castling_moves(move_list)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::attacks::RAYS;
    use crate::board::Board;
    use crate::board::movegen::MoveGenKind;
    use crate::search::data::SearchData;
    use crate::types::Square::{self, *};
    use crate::types::{Move, MoveKind, MoveList};

    #[test]
    fn test_is_attacked() {
        let mut board = Board::from_fen("7k/8/8/3p4/8/8/5N2/K7 w - - 0 1").unwrap();
        board.update_all_threats();
        board.state.threats.print_board();

        assert!(board.is_attacked(C4));
        assert!(board.is_attacked(E4));
        assert!(!board.is_attacked(F2));

        let mut board2 = Board::from_fen("6Q1/8/2R5/8/5b2/kq6/8/6K1 w - - 0 1").unwrap();
        board2.update_all_threats();
        let threats = board2.state.threats;
        threats.print_board();

        assert!(board2.is_attacked(C3));
        assert!(board2.is_attacked(A2));
        assert!(board2.is_attacked(H6));
        assert!(!board2.is_attacked(F5));
    }

    #[test]
    fn test_move_create() {
        let from = A2;
        let to = A4;
        let kind = MoveKind::DoublePawn;

        let m = Move::new(from, to, kind);
        println!("{:?}, {:?}, {:?}", m.get_from(), m.get_to(), m.get_kind());
    }

    #[test]
    fn test_legal_moves() {
        let data = SearchData::default();
        let mut move_list = MoveList::new();

        data.board.append_moves(MoveGenKind::All, &mut move_list);
        println!("{move_list}");
        assert_eq!(move_list.len(), 20);

        let data = SearchData {
            board: Board::from_fen("rnb1kbnr/pp1ppppp/2p5/q7/8/3P4/PPPBPPPP/RN1QKBNR w KQkq - 2 3")
                .unwrap(),
            ..Default::default()
        };

        data.board.state.threats.print_board();
        let mut move_list = MoveList::new();
        data.board.append_moves(MoveGenKind::All, &mut move_list);
        println!("{move_list}");
        RAYS[Square::D2 as usize][Square::E1 as usize].print_board();
    }
}
