use crate::board::Board;
use crate::board::movegen::MoveGenKind;
use crate::types::{Castling, File, HOME_RANK, Move, Piece, Side, Square, pieces};

#[derive(Debug)]
pub struct FenParseError;

impl From<pieces::InvalidPiece> for FenParseError {
    fn from(_: pieces::InvalidPiece) -> Self {
        FenParseError
    }
}

// Starting Position: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
// [pieces] [turn to move] [castling rights] [enpassant] [halfmove clock] [fullmove clock]

// FEN String Parser
impl Board {
    pub fn from_fen(fen_string: &str) -> Result<Self, FenParseError> {
        let mut board = Board::new();
        let mut fen = fen_string.split(" ");
        let piece_string = fen.next().ok_or(FenParseError)?;
        let ranks = piece_string.split('/');

        for (rank, r_str) in ranks.rev().enumerate() {
            let mut file: usize = 0;
            for p in r_str.chars() {
                if let Some(num) = p.to_digit(10) {
                    file += num as usize;
                    continue;
                }

                let side = if p.is_ascii_uppercase() {
                    Side::White
                } else {
                    Side::Black
                };
                let piece = Piece::from_char(p)?;
                let square = Square::from_rank_and_file(rank, file);
                board.place_piece(side, piece, square);

                file += 1;
            }
        }

        let turn = fen.next().ok_or(FenParseError)?;
        match turn {
            "w" => board.state.side_to_move = Side::White,
            "b" => board.state.side_to_move = Side::Black,
            _ => return Err(FenParseError),
        }

        let castling_rights = fen.next().ok_or(FenParseError)?;
        board.parse_castling(castling_rights);

        if let Some(enpassant) = fen.next()
            && let Ok(square) = Square::try_from(enpassant)
        {
            board.state.enpassant = Some(square);
        }

        if let Some(half_move) = fen.next()
            && let Ok(i) = half_move.parse::<u8>()
        {
            board.state.half_move_clock = i;
        }

        if let Some(full_move) = fen.next()
            && let Ok(i) = full_move.parse::<usize>()
        {
            board.state.full_move = i;
        }

        if board.state.side_to_move == Side::Black {
            board.state.keys.toggle_side();
        }

        board
            .state
            .keys
            .toggle_castling(board.state.castling_rights);
        if let Some(square) = board.state.enpassant {
            board.state.keys.toggle_en_passant(square);
        }

        board.update_all_threats();
        board.update_en_passant();
        board.game_history.push(board.state.keys.full);

        Ok(board)
    }

    pub fn parse_move(&self, move_string: &str) -> Result<Move, &str> {
        let from = Square::try_from(&move_string[0..2]).unwrap();
        let to = Square::try_from(&move_string[2..4]).unwrap();
        let mut promotion_piece: Option<Piece> = None;
        if move_string.len() > 4 {
            match &move_string[4..] {
                "n" => promotion_piece = Some(Piece::Knight),
                "b" => promotion_piece = Some(Piece::Bishop),
                "r" => promotion_piece = Some(Piece::Rook),
                "q" => promotion_piece = Some(Piece::Queen),
                _ => (),
            }
        }

        let move_list = self.generate_moves(MoveGenKind::All);
        if let Some(m) = move_list.iter().find(|e| {
            e.mv.get_from() == from
                && e.mv.get_to() == to
                && e.mv.get_promoted_piece() == promotion_piece
        }) {
            Ok(m.mv)
        } else {
            Err("Invalid move string")
        }
    }

    fn parse_castling(&mut self, castling_rights: &str) {
        for c in castling_rights.chars() {
            if matches!(c.to_ascii_uppercase(), 'A'..='H') {
                let side = if c.is_ascii_uppercase() {
                    Side::White
                } else {
                    Side::Black
                };
                let file = File::from(c.to_ascii_uppercase() as u8 - b'A');
                let rook_square = (file.to_bb() & HOME_RANK[side].to_bb())
                    .least_sig_bit()
                    .unwrap();
                let king_square = self.get_king_square(side);
                let dir = if rook_square > king_square {
                    Castling::KING_SIDE
                } else {
                    Castling::QUEEN_SIDE
                };
                self.castling_rooks[side][dir] = rook_square;
                let mask = Castling::KINDS[side][dir] as u8;
                self.state.castling_rights.set(mask);
            } else if matches!(c.to_ascii_uppercase(), 'K' | 'Q') {
                self.state.castling_rights.set(Castling::from(c) as u8);
            } else {
                continue;
            }
        }
    }

    // Starting Position: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    // [pieces] [turn to move] [castling rights] [enpassant] [halfmove clock] [fullmove clock]
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                let p = self.get_piece_at_square(Square::from_rank_and_file(rank, file));
                if let Some((side, piece)) = p {
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                    }
                    empty = 0;
                    fen.push(piece.to_char(side));
                } else {
                    empty += 1;
                }
            }

            if empty > 0 {
                fen.push_str(&empty.to_string());
            }

            if rank != 0 {
                fen.push('/');
            }
        }

        fen.push(' ');
        fen.push_str(&self.state.side_to_move.to_string());
        fen.push(' ');
        fen.push_str(&self.state.castling_rights.to_string(self));
        fen.push(' ');
        let ep_string = match self.state.enpassant {
            Some(square) => square.to_string(),
            None => "-".to_string(),
        };

        fen.push_str(&ep_string);
        fen.push(' ');
        fen.push_str(&self.state.half_move_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.state.full_move.to_string());

        fen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants::STARTING_FEN;

    #[test]
    fn test_from_fen() {
        let board = Board::from_fen(STARTING_FEN).unwrap();
        println!("{board}");

        let board3 =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();
        println!("{board3}");

        let board4 =
            Board::from_fen("rnbqkb1r/pp1p1pPp/8/2p1pP2/1P1P4/3P3P/P1P1P3/RNBQKBNR w KQkq e6 0 1")
                .unwrap();
        println!("{board4}");

        let board5 =
            Board::from_fen("r2q1rk1/ppp2ppp/2n1bn2/2b1p3/3pP3/3P1NPP/PPP1NPB1/R1BQ1RK1 b - - 0 9")
                .unwrap();
        println!("{board5}");

        let board5 =
            Board::from_fen("rnbqkbnr/pp3ppp/4p3/2pp4/3P4/2P2N2/PP2PPPP/RNBQKB1R w KQkq c6 0 4")
                .unwrap();
        println!("{board5}");

        let board6 =
            Board::from_fen("rnb1kbnr/pp1q1pp1/4p2p/2p1N3/3Pp3/2P5/PP2BPPP/RNBQK1R1 b Qkq - 1 7")
                .unwrap();
        println!("{board6}");
        println!("Half move: {}", board6.state.half_move_clock);
        println!("Full move: {}", board6.state.full_move);
    }

    #[test]
    fn test_to_fen() {
        let board = Board::from_fen(STARTING_FEN).unwrap();
        assert_eq!(STARTING_FEN, board.to_fen());

        let board =
            Board::from_fen("r2q1rk1/ppp2ppp/2n1bn2/2b1p3/3pP3/3P1NPP/PPP1NPB1/R1BQ1RK1 b - - 0 9")
                .unwrap();
        assert_eq!(
            "r2q1rk1/ppp2ppp/2n1bn2/2b1p3/3pP3/3P1NPP/PPP1NPB1/R1BQ1RK1 b - - 0 9",
            board.to_fen()
        );

        let board = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 ").unwrap();
        assert_eq!("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", board.to_fen());
    }

    #[test]
    fn test_frc_fen() {
        let board =
            Board::from_fen("qrkbbnnr/pppppppp/8/8/8/8/PPPPPPPP/RKRQNNBB w CAhb - 0 1").unwrap();
        println!("{:?}", board.castling_rooks);
    }
}
