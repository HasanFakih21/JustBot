use crate::attacks::*;
use crate::tools::magics::*;
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
    pub mailbox: [Option<(Side, Piece)>; 64],
    pub side_to_move: Side,
    pub enpassant: Option<Square>,
    pub castling_rights: CastlingRights,
    pub threats: BitBoard,
    pub pinned: [BitBoard; 2],
    pub checkers: BitBoard,
    pub checking_squares: [BitBoard; 6],

    pub half_move_clock: u8,
    pub full_move: usize,
    pub hash: u64,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
            pieces: [BitBoard(0); 6],
            occupancies: [BitBoard(0); 2],
            mailbox: [None; 64],
            side_to_move: Side::White,
            enpassant: None,
            castling_rights: CastlingRights::new(),
            threats: BitBoard(0),
            pinned: [BitBoard(0); 2],
            checkers: BitBoard(0),
            checking_squares: [BitBoard(0); 6],

            half_move_clock: 0,
            full_move: 0,
            hash: 0,
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
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

//Little-Endian Rank-File Mapping
impl Board {
    pub fn new() -> Self {
        Board {
            state_stack: Vec::new(),
            state: BoardState::new(),
            game_history: Vec::new(),
        }
    }

    pub fn is_attacked(&self, square: Square) -> bool {
        let threats = self.state.threats;
        threats.contains(square)
    }

    pub fn get_king_square(&self, side: Side) -> Square {
        debug_assert!(self.get_piece_bb(side, Piece::King).0 != 0, "{}", self);
        self.get_piece_bb(side, Piece::King)
            .least_sig_bit()
            .unwrap()
    }

    pub fn is_direct_check(&self, m: Move) -> bool {
        let (_, piece) = self.get_piece_at_square(m.get_from()).unwrap();
        self.state.checking_squares[piece as usize].contains(m.get_to())
    }

    pub fn update_all_threats(&mut self) {
        let side = self.state.side_to_move.other();
        let stm = self.state.side_to_move;
        let occ_bb = self.get_all_occupancy() ^ self.get_piece_bb(stm, Piece::King);
        let king_square = self.get_king_square(stm);
        let other_king_square = self.get_king_square(stm.other());

        self.state.checking_squares[Piece::Pawn as usize] =
            self.get_pawn_attacks(other_king_square, stm.other());
        self.state.checking_squares[Piece::Knight as usize] =
            self.get_knight_attacks(other_king_square);
        self.state.checking_squares[Piece::Bishop as usize] =
            self.get_bishop_attacks(other_king_square, self.get_all_occupancy());
        self.state.checking_squares[Piece::Rook as usize] =
            self.get_rook_attacks(other_king_square, self.get_all_occupancy());
        self.state.checking_squares[Piece::Queen as usize] = self.state.checking_squares
            [Piece::Rook as usize]
            | self.state.checking_squares[Piece::Bishop as usize];

        self.state.threats = self.pawn_attacks_setwise(side)
            | self.knight_attacks_setwise(side)
            | self.bishop_attacks_setwise(side, occ_bb)
            | self.rook_attacks_setwise(side, occ_bb)
            | self.queen_attacks_setwise(side, occ_bb)
            | self.get_king_attacks(self.get_king_square(side));

        let pawn_attackers = self.get_piece_bb(stm.other(), Piece::Pawn);
        let knight_attackers = self.get_piece_bb(stm.other(), Piece::Knight);
        self.state.checkers = (self.get_pawn_attacks(king_square, stm) & pawn_attackers)
            | (self.get_knight_attacks(king_square) & knight_attackers);
        self.state.pinned[stm as usize] = BitBoard(0);

        let bishop_queens = self.get_piece_bb(stm.other(), Piece::Bishop)
            | self.get_piece_bb(stm.other(), Piece::Queen);
        let rook_queens = self.get_piece_bb(stm.other(), Piece::Rook)
            | self.get_piece_bb(stm.other(), Piece::Queen);

        let opp_occ = self.state.occupancies[stm.other() as usize];
        let diag_attackers =
            bishop_queens & self.get_bishop_attacks(king_square, opp_occ) & opp_occ;
        let straight_attackers = rook_queens & self.get_rook_attacks(king_square, opp_occ) & occ_bb;

        for square in (diag_attackers | straight_attackers).iter() {
            let blockers = BETWEEN[square as usize][king_square as usize]
                & self.state.occupancies[stm as usize];
            let pieces_betweeen = blockers.count_bits();
            if pieces_betweeen == 1 {
                self.state.pinned[stm as usize] |= blockers;
            } else if pieces_betweeen == 0 {
                self.state.checkers.set_bit(square);
            }
        }
    }

    pub const fn get_piece_bb(&self, side: Side, piece: Piece) -> BitBoard {
        BitBoard(self.state.pieces[piece as usize].0 & self.state.occupancies[side as usize].0)
    }

    pub const fn get_piece_at_square(&self, square: Square) -> Option<(Side, Piece)> {
        self.state.mailbox[square as usize]
    }

    pub fn place_piece(&mut self, side: Side, piece: Piece, square: Square) {
        //Bitboards
        self.state.pieces[piece as usize].set_bit(square);
        self.state.occupancies[side as usize].set_bit(square);
        //Mailbox
        self.state.mailbox[square as usize] = Some((side, piece));
        //Zobrist Hash
        self.state.hash ^= ZOBRIST.get_piece_num(side, piece, square);
    }

    pub fn remove_piece(&mut self, side: Side, piece: Piece, square: Square) {
        //Bitboards
        self.state.pieces[piece as usize].clear_bit(square);
        self.state.occupancies[side as usize].clear_bit(square);
        //Mailbox
        self.state.mailbox[square as usize] = None;
        //Zobrist Hash
        self.state.hash ^= ZOBRIST.get_piece_num(side, piece, square);
    }

    //Needs fixing
    pub fn detect_repetitions(&self) -> usize {
        let half_moves = self.state.half_move_clock as usize;
        let mut count = 0;

        if self.game_history.len() < half_moves {
            return 0;
        }

        let last_halfmove_ply = self.game_history.len() - half_moves;
        for position in self.game_history[last_halfmove_ply..].iter() {
            if self.state.hash == *position {
                count += 1
            }
        }

        count
    }

    pub fn get_piece_attack(&self, side: Side, square: Square, piece: Piece) -> BitBoard {
        match piece {
            Piece::Pawn => self.get_pawn_attacks(square, side),
            Piece::Knight => self.get_knight_attacks(square),
            Piece::Bishop => self.get_bishop_attacks(square, self.get_all_occupancy()),
            Piece::Rook => self.get_rook_attacks(square, self.get_all_occupancy()),
            Piece::Queen => self.get_queen_attacks(square, self.get_all_occupancy()),
            Piece::King => self.get_king_attacks(square),
        }
    }

    pub const fn get_pawn_attacks(&self, square: Square, side: Side) -> BitBoard {
        PAWN_ATTACKS[side as usize][square as usize]
    }

    pub fn pawn_attacks_setwise(&self, side: Side) -> BitBoard {
        let pawns = self.get_piece_bb(side, Piece::Pawn);
        let (top_left, top_right) = match side {
            Side::White => (7, 9),
            Side::Black => (-9, -7),
        };

        (!A & pawns).shift(top_left) | (!H & pawns).shift(top_right)
    }

    pub const fn get_knight_attacks(&self, square: Square) -> BitBoard {
        KNIGHT_ATTACKS[square as usize]
    }

    pub fn knight_attacks_setwise(&self, side: Side) -> BitBoard {
        let knights = self.get_piece_bb(side, Piece::Knight);

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

    pub const fn get_king_attacks(&self, square: Square) -> BitBoard {
        KING_ATTACKS[square as usize]
    }

    pub fn get_bishop_attacks(&self, square: Square, board_occupancy: BitBoard) -> BitBoard {
        let occupancy = board_occupancy & BISHOP_MASKS[square as usize];
        let magic_index = get_magic_index(
            occupancy,
            BISHOP_OCCUPANCY_BIT_COUNTS[square as usize],
            BISHOP_MAGIC_NUMBERS[square as usize],
        );

        let offset = (square as usize * 512) + magic_index;

        BISHOP_ATTACKS[offset]
    }

    pub fn bishop_attacks_setwise(&self, side: Side, occ_bb: BitBoard) -> BitBoard {
        let bishops = self.get_piece_bb(side, Piece::Bishop);
        let mut attacks = BitBoard(0);
        for square in bishops.iter() {
            attacks |= self.get_bishop_attacks(square, occ_bb);
        }

        attacks
    }

    pub fn get_rook_attacks(&self, square: Square, board_occupancy: BitBoard) -> BitBoard {
        let occupancy = board_occupancy & ROOK_MASKS[square as usize];
        let magic_index = get_magic_index(
            occupancy,
            ROOK_OCCUPANCY_BIT_COUNTS[square as usize],
            ROOK_MAGIC_NUMBERS[square as usize],
        );

        let offset = (square as usize * 4096) + magic_index;

        ROOK_ATTACKS[offset]
    }

    pub fn rook_attacks_setwise(&self, side: Side, occ_bb: BitBoard) -> BitBoard {
        let rooks = self.get_piece_bb(side, Piece::Rook);
        let mut attacks = BitBoard(0);
        for square in rooks.iter() {
            attacks |= self.get_rook_attacks(square, occ_bb);
        }

        attacks
    }

    pub fn get_queen_attacks(&self, square: Square, board_occupancy: BitBoard) -> BitBoard {
        self.get_bishop_attacks(square, board_occupancy)
            | self.get_rook_attacks(square, board_occupancy)
    }

    pub fn queen_attacks_setwise(&self, side: Side, occ_bb: BitBoard) -> BitBoard {
        let queens = self.get_piece_bb(side, Piece::Queen);

        let mut attacks = BitBoard(0);
        for square in queens.iter() {
            attacks |=
                self.get_rook_attacks(square, occ_bb) | self.get_bishop_attacks(square, occ_bb);
        }

        attacks
    }

    pub fn get_all_occupancy(&self) -> BitBoard {
        self.state.occupancies[Side::White as usize] | self.state.occupancies[Side::Black as usize]
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::from("\n");
        for rank in (0..8).rev() {
            output.push_str(&format!("{}   ", 1 + rank));
            for file in 0..8 {
                let square = Square::from_rank_and_file(rank, file);
                let piece: Option<(Side, Piece)> = self.get_piece_at_square(square);

                if let Some(p) = piece {
                    let mut s = format!(" {} ", p.1);
                    if let Side::Black = p.0 {
                        s = s.to_lowercase();
                    }
                    output.push_str(&s);
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
    use crate::search::data::SearchData;

    use super::*;

    #[test]
    fn test_get_rook_attack() {
        let board = Board::new();
        let mut occ = BitBoard(0);

        board.get_rook_attacks(Square::A6, occ).print_board();

        occ.set_bit(Square::E3);
        occ.set_bit(Square::G5);
        occ.set_bit(Square::G3);

        board.get_rook_attacks(Square::G3, occ).print_board();
    }

    #[test]
    fn test_get_bishop_attack() {
        let board = Board::new();
        let mut occ = BitBoard(0);

        board.get_bishop_attacks(Square::A3, occ).print_board();

        occ.set_bit(Square::D6);
        board.get_bishop_attacks(Square::G3, occ).print_board();
    }

    #[test]
    fn test_get_queen_attack() {
        let board = Board::new();
        let mut occ = BitBoard(0);

        board.get_queen_attacks(Square::A6, occ).print_board();

        occ.set_bit(Square::E3);
        occ.set_bit(Square::G5);
        occ.set_bit(Square::G3);
        occ.set_bit(Square::D6);

        board.get_queen_attacks(Square::G3, occ).print_board();
        board.get_queen_attacks(Square::E4, occ).print_board();
    }

    #[test]
    fn test_board_occupancy() {
        let mut board = Board::from_fen(STARTING_FEN).unwrap();
        board.remove_piece(Side::White, Piece::Pawn, Square::A2);
        board.get_all_occupancy().print_board();
        board.state.occupancies[Side::Black as usize].print_board();
        board.state.occupancies[Side::White as usize].print_board();
    }

    #[test]
    fn test_full_board_print() {
        let board = Board::new();
        println!("{board}");
    }

    #[test]
    fn test_pawn_attacks_setwise() {
        let data = SearchData {
            board: Board::from_fen(
                "rnbqkb1r/pp3p2/4pnpp/1p1p2N1/1Q1P4/BP2P3/P1PN1PPP/R3K2R b KQkq - 0 1",
            )
            .unwrap(),
            ..Default::default()
        };

        let pawn_attacks = data.board.pawn_attacks_setwise(Side::Black);
        pawn_attacks.print_board();
        assert_eq!(pawn_attacks.count_bits(), 12);
    }

    #[test]
    fn test_knight_attacks_setwise() {
        let data = SearchData {
            board: Board::from_fen(
                "rnbqkb1r/pp3p2/4pnpp/1p1p2N1/1Q1P4/BP2P3/P1PN1PPP/R3K2R b KQkq - 0 1",
            )
            .unwrap(),
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
        data.board.state.pinned[stm as usize].print_board();

        let mut data = SearchData {
            board: Board::from_fen("8/2K5/8/5k2/1n3p2/8/8/5Q2 b - - 0 1").unwrap(),
            ..Default::default()
        };

        data.board.update_all_threats();
        let stm = data.board.state.side_to_move;
        data.board.state.pinned[stm as usize].print_board();
    }

    #[test]
    fn test_checking_squares() {
        let data = SearchData {
            board: Board::from_fen(
                "rnbqk2r/pp3p2/4pnpp/1p1p2N1/1b1P4/BP2P2P/P1PN1PP1/R3K2R b KQkq - 0 2",
            )
            .unwrap(),
            ..Default::default()
        };

        let m = data.board.parse_move("b4d2").unwrap();
        assert!(data.board.is_direct_check(m));

        let m = data.board.parse_move("b4a3").unwrap();
        assert!(!data.board.is_direct_check(m));
    }
}
