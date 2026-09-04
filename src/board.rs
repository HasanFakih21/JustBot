use crate::attacks::*;
use crate::lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks};
use crate::types::keys::Keys;
use crate::types::*;
use std::fmt::Display;

pub mod legal;
pub mod makemove;
pub mod movegen;
pub mod parser;
pub mod see;

#[derive(Debug, Clone, PartialEq)]
pub struct BoardState {
    pub pieces: [BitBoard; 6],
    pub occupancies: [BitBoard; 2],
    pub mailbox: [OptionPiece<SidedPiece>; 64],
    pub side_to_move: Side,
    pub enpassant: Option<Square>,
    pub castling_rights: CastlingRights,
    pub threats: BitBoard,
    pub pinned: [BitBoard; 2],
    pub pinners: [BitBoard; 2],
    pub checkers: BitBoard,
    pub checking_squares: [BitBoard; 6],
    pub plies_from_null: usize,

    pub half_move_clock: u8,
    pub full_move: usize,
    pub keys: Keys,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
            pieces: [BitBoard(0); 6],
            occupancies: [BitBoard(0); 2],
            mailbox: [OptionPiece::None; 64],
            side_to_move: Side::White,
            enpassant: None,
            castling_rights: CastlingRights::new(),
            threats: BitBoard(0),
            pinned: [BitBoard(0); 2],
            pinners: [BitBoard(0); 2],
            checkers: BitBoard(0),
            checking_squares: [BitBoard(0); 6],
            plies_from_null: 0,

            half_move_clock: 0,
            full_move: 0,
            keys: Keys::default(),
        }
    }
}

impl Default for BoardState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Board {
    pub state: BoardState,
    pub state_stack: Vec<BoardState>,
    pub game_history: Vec<u64>,
    pub castling_rooks: [[Square; 2]; 2],
    pub frc: bool,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

// Little-Endian Rank-File Mapping
impl Board {
    pub fn new() -> Self {
        Board {
            state_stack: Vec::new(),
            state: BoardState::new(),
            game_history: Vec::new(),
            castling_rooks: CASTLING_ROOK_SQAURES,
            frc: false,
        }
    }

    pub fn threats(&self) -> BitBoard {
        self.state.threats
    }

    pub fn occ(&self, side: Side) -> BitBoard {
        self.state.occupancies[side]
    }

    pub fn halfmove_bucket(&self) -> usize {
        (self.state.half_move_clock.saturating_sub(8) as usize / 8).min(15)
    }

    pub fn hash(&self) -> u64 {
        self.state.keys.full ^ ZOBRIST.get_halfmove_num(self.halfmove_bucket())
    }

    pub fn is_attacked(&self, square: Square) -> bool {
        let threats = self.threats();
        threats.contains(square)
    }

    pub fn king_square(&self, side: Side) -> Square {
        debug_assert!(self.piece_bb(side, Piece::King).0 != 0, "{}", self);
        self.piece_bb(side, Piece::King).least_sig_bit().unwrap()
    }

    pub fn is_direct_check(&self, m: Move) -> bool {
        let piece = self.piece_at_square(m.from()).unwrap().kind();
        self.state.checking_squares[piece].contains(m.to())
    }

    pub fn update_all_threats(&mut self) {
        let stm = self.state.side_to_move;
        let occ_bb = self.all_occupancy() ^ self.piece_bb(stm, Piece::King);

        self.state.threats = self.pawn_attacks_setwise(!stm)
            | self.knight_attacks_setwise(!stm)
            | self.bishop_attacks_setwise(!stm, occ_bb)
            | self.rook_attacks_setwise(!stm, occ_bb)
            | self.queen_attacks_setwise(!stm, occ_bb)
            | king_attacks(self.king_square(!stm));

        self.state.pinned = [BitBoard(0); 2];
        self.state.pinners = [BitBoard(0); 2];

        for side in [Side::White, Side::Black] {
            let king_square = self.king_square(side);
            if side == stm {
                let pawn_attackers = self.piece_bb(!stm, Piece::Pawn);
                let knight_attackers = self.piece_bb(!stm, Piece::Knight);
                self.state.checkers = (pawn_attacks(king_square, stm) & pawn_attackers)
                    | (knight_attacks(king_square) & knight_attackers);
            } else {
                self.state.checking_squares[Piece::Pawn] = pawn_attacks(king_square, !stm);
                self.state.checking_squares[Piece::Knight] = knight_attacks(king_square);
                self.state.checking_squares[Piece::Bishop] = bishop_attacks(king_square, self.all_occupancy());
                self.state.checking_squares[Piece::Rook] = rook_attacks(king_square, self.all_occupancy());
                self.state.checking_squares[Piece::Queen] =
                    self.state.checking_squares[Piece::Rook] | self.state.checking_squares[Piece::Bishop];
            }

            let opp_occ = self.occ(!side);
            let diagonal = (self.piece_bb(!side, Piece::Bishop) | self.piece_bb(!side, Piece::Queen))
                & bishop_attacks(king_square, opp_occ);
            let orthogonal = (self.piece_bb(!side, Piece::Rook) | self.piece_bb(!side, Piece::Queen))
                & rook_attacks(king_square, opp_occ);

            for square in diagonal | orthogonal {
                let blockers = BETWEEN[square][king_square] & self.occ(side);

                let pieces_betweeen = blockers.count_bits();
                if pieces_betweeen == 1 {
                    self.state.pinned[side] |= blockers;
                    self.state.pinners[!side].set_bit(square);
                } else if pieces_betweeen == 0 {
                    self.state.checkers.set_bit(square);
                }
            }
        }
    }

    pub fn piece_bb(&self, side: Side, piece: Piece) -> BitBoard {
        BitBoard(self.state.pieces[piece].0 & self.occ(side).0)
    }

    pub fn piece_at_square(&self, square: Square) -> OptionPiece<SidedPiece> {
        self.state.mailbox[square]
    }

    pub fn place_piece(&mut self, side: Side, piece: Piece, square: Square) {
        // Bitboards
        self.state.pieces[piece].set_bit(square);
        self.state.occupancies[side].set_bit(square);
        // Mailbox
        self.state.mailbox[square] = OptionPiece::Some(SidedPiece::from(side, piece));
        // Zobrist Hash
        self.state.keys.toggle(side, piece, square);
    }

    pub fn remove_piece(&mut self, side: Side, piece: Piece, square: Square) {
        // Bitboards
        self.state.pieces[piece].clear_bit(square);
        self.state.occupancies[side].clear_bit(square);
        // Mailbox
        self.state.mailbox[square] = OptionPiece::None;
        // Zobrist Hash
        self.state.keys.toggle(side, piece, square);
    }

    pub fn pawn_attacks_setwise(&self, side: Side) -> BitBoard {
        let pawns = self.piece_bb(side, Piece::Pawn);
        let (top_left, top_right) = match side {
            Side::White => (7, 9),
            Side::Black => (-9, -7),
        };

        (!A & pawns).shift(top_left) | (!H & pawns).shift(top_right)
    }

    pub fn knight_attacks_setwise(&self, side: Side) -> BitBoard {
        let knights = self.piece_bb(side, Piece::Knight);

        let not_a = knights & !A;
        let not_ab = knights & !AB;
        let not_h = knights & !H;
        let not_hg = knights & !HG;

        not_a.shift(15)
            | not_ab.shift(6)
            | not_a.shift(-17)
            | not_ab.shift(-10)
            | not_h.shift(17)
            | not_hg.shift(10)
            | not_h.shift(-15)
            | not_hg.shift(-6)
    }

    pub fn bishop_attacks_setwise(&self, side: Side, occ_bb: BitBoard) -> BitBoard {
        let bishops = self.piece_bb(side, Piece::Bishop);
        let mut attacks = BitBoard(0);
        for square in bishops {
            attacks |= bishop_attacks(square, occ_bb);
        }

        attacks
    }

    pub fn rook_attacks_setwise(&self, side: Side, occ_bb: BitBoard) -> BitBoard {
        let rooks = self.piece_bb(side, Piece::Rook);
        let mut attacks = BitBoard(0);
        for square in rooks {
            attacks |= rook_attacks(square, occ_bb);
        }

        attacks
    }

    pub fn queen_attacks_setwise(&self, side: Side, occ_bb: BitBoard) -> BitBoard {
        let queens = self.piece_bb(side, Piece::Queen);

        let mut attacks = BitBoard(0);
        for square in queens {
            attacks |= rook_attacks(square, occ_bb) | bishop_attacks(square, occ_bb);
        }

        attacks
    }

    pub fn all_occupancy(&self) -> BitBoard {
        self.occ(Side::White) | self.occ(Side::Black)
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::from("\n");
        for rank in (0..8).rev() {
            output.push_str(&format!("{}   ", 1 + rank));
            for file in 0..8 {
                let square = Square::from_rank_and_file(rank, file);
                let piece: OptionPiece<SidedPiece> = self.piece_at_square(square);
                if let OptionPiece::Some(p) = piece {
                    output.push_str(&format!(" {} ", p));
                } else {
                    output.push_str(" . ");
                }
            }
            output.push('\n');
        }
        output.push_str("\n     A  B  C  D  E  F  G  H\n");
        output.push('\n');
        output.push_str(&self.to_fen());
        write!(f, "{}", output)
    }
}

#[cfg(test)]
mod tests {
    use crate::{lookup::queen_attacks, search::data::SearchData};

    use super::*;

    #[test]
    fn test_get_rook_attack() {
        let mut occ = BitBoard(0);

        rook_attacks(Square::A6, occ).print_board();

        occ.set_bit(Square::E3);
        occ.set_bit(Square::G5);
        occ.set_bit(Square::G3);

        rook_attacks(Square::G3, occ).print_board();
    }

    #[test]
    fn test_get_bishop_attack() {
        let mut occ = BitBoard(0);

        bishop_attacks(Square::A3, occ).print_board();

        occ.set_bit(Square::D6);
        bishop_attacks(Square::G3, occ).print_board();
    }

    #[test]
    fn test_get_queen_attack() {
        let mut occ = BitBoard(0);

        queen_attacks(Square::A6, occ).print_board();

        occ.set_bit(Square::E3);
        occ.set_bit(Square::G5);
        occ.set_bit(Square::G3);
        occ.set_bit(Square::D6);

        queen_attacks(Square::G3, occ).print_board();
        queen_attacks(Square::E4, occ).print_board();
    }

    #[test]
    fn test_board_occupancy() {
        let mut board = Board::from_fen(STARTING_FEN).unwrap();
        board.remove_piece(Side::White, Piece::Pawn, Square::A2);
        board.all_occupancy().print_board();
        board.state.occupancies[Side::Black].print_board();
        board.state.occupancies[Side::White].print_board();
    }

    #[test]
    fn test_full_board_print() {
        let board = Board::new();
        println!("{board}");
    }

    #[test]
    fn test_pawn_attacks_setwise() {
        let data = SearchData {
            board: Board::from_fen("rnbqkb1r/pp3p2/4pnpp/1p1p2N1/1Q1P4/BP2P3/P1PN1PPP/R3K2R b KQkq - 0 1").unwrap(),
            ..Default::default()
        };

        let pawn_attacks = data.board.pawn_attacks_setwise(Side::Black);
        pawn_attacks.print_board();
        assert_eq!(pawn_attacks.count_bits(), 12);
    }

    #[test]
    fn test_knight_attacks_setwise() {
        let data = SearchData {
            board: Board::from_fen("rnbqkb1r/pp3p2/4pnpp/1p1p2N1/1Q1P4/BP2P3/P1PN1PPP/R3K2R b KQkq - 0 1").unwrap(),
            ..Default::default()
        };

        let knight_attacks = data.board.knight_attacks_setwise(Side::Black);
        knight_attacks.print_board();
        assert_eq!(knight_attacks.count_bits(), 10);
    }

    #[test]
    fn test_pinned_and_checkers() {
        let mut data = SearchData {
            board: Board::from_fen("8/8/1Q3K2/8/1n6/1k6/8/8 b - - 0 1").unwrap(),
            ..Default::default()
        };

        data.board.update_all_threats();
        let stm = data.board.state.side_to_move;
        data.board.state.pinned[stm].print_board();

        let mut data = SearchData {
            board: Board::from_fen("8/2K5/8/5k2/1n3p2/8/8/5Q2 b - - 0 1").unwrap(),
            ..Default::default()
        };

        data.board.update_all_threats();
        let stm = data.board.state.side_to_move;
        data.board.state.pinned[stm].print_board();
    }

    #[test]
    fn test_checking_squares() {
        let data = SearchData {
            board: Board::from_fen("rnbqk2r/pp3p2/4pnpp/1p1p2N1/1b1P4/BP2P2P/P1PN1PP1/R3K2R b KQkq - 0 2").unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("b4d2").unwrap();
        assert!(data.board.is_direct_check(m));

        let m = data.board.parse_move("b4a3").unwrap();
        assert!(!data.board.is_direct_check(m));
    }
}
