use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::board::Board;
use crate::nnue::{Accumulator, NNUE};
use crate::search::time::{TimeManager, TimeSettings};
use crate::types::plytable::PlyTable;
use crate::types::{
    ContinuationHistory, KING_SIDE_ROOK_BLACK, KING_SIDE_ROOK_WHITE, Move, MoveKind, MoveList,
    NoisyHistory, Piece, QUEEN_SIDE_ROOK_BLACK, QUEEN_SIDE_ROOK_WHITE, STARTING_FEN, Score, Side,
    Square, to_file_bb,
};
use crate::types::{QuietHistory, TranspositionTable};

#[derive(Debug)]
pub struct Status(AtomicBool);

impl Status {
    pub const RUNNING: bool = true;
    pub const STOPPED: bool = false;

    pub fn stop(&self) {
        self.0.store(Self::STOPPED, Ordering::Relaxed);
    }

    pub fn run(&self) {
        self.0.store(Self::RUNNING, Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct SharedData {
    pub tt: TranspositionTable,
    pub total_nodes: AtomicUsize,
    pub status: Status,
}

impl SharedData {
    pub fn get_total_nodes_searched(&self) -> usize {
        self.total_nodes.load(Ordering::Acquire)
    }

    pub fn increase_nodes(&self, nodes: usize) {
        self.total_nodes.fetch_add(nodes, Ordering::Relaxed);
    }

    pub fn clear_node_count(&self) {
        self.total_nodes.store(0, Ordering::Release);
    }
}

impl Default for SharedData {
    fn default() -> Self {
        Self {
            tt: TranspositionTable::default(),
            total_nodes: AtomicUsize::new(0),
            status: Status(AtomicBool::new(Status::RUNNING)),
        }
    }
}

#[derive(Debug)]
pub struct SearchData {
    pub id: usize,
    pub best_move: Option<Move>,

    pub shared: Arc<SharedData>,
    pub pv: Vec<MoveList>,
    pub board: Board,
    pub time: TimeManager,
    pub report: bool,
    pub ply_table: Box<PlyTable>,
    pub nodes: u64,

    pub quiet_history: QuietHistory,
    pub noisy_history: NoisyHistory,
    pub conthistory: ContinuationHistory,

    pub white_features: Accumulator,
    pub black_features: Accumulator,
}

impl SearchData {
    pub fn new(shared: Arc<SharedData>, id: usize) -> Self {
        SearchData {
            id,
            best_move: None,
            shared,
            pv: vec![MoveList::new(); 128],
            board: Board::from_fen(STARTING_FEN).unwrap(),
            time: TimeManager::new(),
            ply_table: PlyTable::new(),
            nodes: 0,

            quiet_history: QuietHistory::new(),
            noisy_history: NoisyHistory::new(),
            conthistory: ContinuationHistory::new(),
            report: true,

            white_features: Accumulator::new(&NNUE),
            black_features: Accumulator::new(&NNUE),
        }
    }

    pub fn increment_nodes(&mut self) {
        self.nodes += 1;
        self.shared.increase_nodes(1);
    }

    pub fn mute(&mut self) {
        self.report = false;
    }

    pub fn report(&mut self) {
        self.report = true;
    }

    pub fn clear_features(&mut self) {
        self.white_features = Accumulator::new(&NNUE);
        self.black_features = Accumulator::new(&NNUE);
    }

    pub fn get_pv(&self) -> &MoveList {
        &self.pv[0]
    }

    pub fn get_best_move(&self) -> Option<Move> {
        self.best_move
    }

    pub fn nodes_per_second(&self) -> usize {
        (self.shared.get_total_nodes_searched() as f32 / self.time.elapsed().as_secs_f32()) as usize
    }

    pub fn start_time(&mut self) {
        self.time.reset_clock();
    }

    pub fn add_pv_move(&mut self, m: Move, ply: isize) {
        self.pv[ply as usize].clear();
        self.pv[ply as usize].push(m);
        for child_m in self.pv[(ply + 1) as usize].clone().iter() {
            self.pv[ply as usize].push(child_m.mv);
        }
    }

    pub fn clear_pv(&mut self, ply: isize) {
        self.pv[ply as usize].clear();
    }

    pub fn get_time_settings(&mut self) -> &mut TimeSettings {
        &mut self.time.settings
    }

    pub fn reset_pv(&mut self) {
        self.pv = vec![MoveList::new(); 128];
    }

    pub fn update_conthistories(&mut self, m: Move, ply: isize, bonus: i32) {
        unsafe {
            self.conthistory.update(
                self.ply_table[ply - 1].conthistory,
                self.board.get_piece_at_square(m.get_from()),
                m.get_to(),
                bonus,
            );

            self.conthistory.update(
                self.ply_table[ply - 2].conthistory,
                self.board.get_piece_at_square(m.get_from()),
                m.get_to(),
                bonus,
            );
        }
    }

    pub fn get_conthistory(&self, m: Move, ply: isize, index: isize) -> i32 {
        unsafe {
            self.conthistory.get(
                self.ply_table[ply - index].conthistory,
                self.board.get_piece_at_square(m.get_from()),
                m.get_to(),
            )
        }
    }

    pub fn print_uci_info(&self, score: i32, depth: i32) {
        //All infos belonging to the pv should be sent together e.g. info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3
        if self.report {
            //Report mate score
            let score_print = if score.abs() > Score::MATE_CUTOFF {
                let num_plies = Score::MATE - score.abs();
                let mate_in = score.signum() * ((num_plies + 1) / 2);
                format!("mate {}", mate_in)
            } else {
                format!("cp {}", score)
            };

            println!(
                "info depth {} time {} score {} nodes {} nps {} pv {} hashfull {}",
                depth - 1,
                self.time.elapsed().as_millis(),
                score_print,
                self.shared.get_total_nodes_searched(),
                self.nodes_per_second(),
                self.get_pv(),
                self.shared.tt.hashfull(),
            );
        }
    }

    //Called before move is made on the board
    pub fn make_move(&mut self, m: Move, ply: isize) {
        let from = m.get_from();
        let to = m.get_to();
        let kind = m.get_kind();
        let stm = self.board.state.side_to_move;

        //Need to toggle off extra captured piece in case of capture
        if kind.is_capture() {
            let capture_square = m.get_capture_square();
            let (_, captured_piece) = self.board.get_piece_at_square(capture_square).unwrap();

            self.toggle_accumulators_off(stm.other(), captured_piece, capture_square);
        }

        //Need to toggle rook in case of castling
        if kind == MoveKind::KingCastle {
            debug_assert!(!(from.to_bb() & to_file_bb(Square::E4)).is_empty());
            let king_rook_square = match stm {
                Side::White => KING_SIDE_ROOK_WHITE,
                Side::Black => KING_SIDE_ROOK_BLACK,
            };

            self.toggle_accumulators_off(stm, Piece::Rook, king_rook_square);
            self.toggle_accumulators_on(stm, Piece::Rook, from.shift(1).unwrap());
        }

        //Need to toggle rook in case of castling
        if kind == MoveKind::QueenCastle {
            debug_assert!(!(from.to_bb() & to_file_bb(Square::E4)).is_empty());
            let queen_rook_square = match stm {
                Side::White => QUEEN_SIDE_ROOK_WHITE,
                Side::Black => QUEEN_SIDE_ROOK_BLACK,
            };

            self.toggle_accumulators_off(stm, Piece::Rook, queen_rook_square);
            self.toggle_accumulators_on(stm, Piece::Rook, from.shift(-1).unwrap());
        }

        let moving_piece = self.board.get_piece_at_square(from).unwrap().1;
        //Need to handle promotions
        if kind.is_promotion() {
            let promotion_piece = m.get_promoted_piece().unwrap();
            self.toggle_accumulators_off(stm, moving_piece, from);
            self.toggle_accumulators_on(stm, promotion_piece, to);
        } else {
            self.toggle_accumulators_off(stm, moving_piece, from);
            self.toggle_accumulators_on(stm, moving_piece, to);
        }

        self.ply_table[ply].m = m;
        self.ply_table[ply].piece = Some((stm, moving_piece));
        self.ply_table[ply].conthistory = self
            .conthistory
            .subtable(Some((stm, moving_piece)), m.get_to());

        self.board.make_move(m)
    }

    //Called after move is already unmade on the board
    pub fn unmake_move(&mut self, m: Move) {
        self.board.unmake_move();

        let from = m.get_from();
        let to = m.get_to();
        let kind = m.get_kind();
        let stm = self.board.state.side_to_move;

        //Need to toggle off extra captured piece in case of capture
        if kind.is_capture() {
            let capture_square = m.get_capture_square();
            let (_, captured_piece) = self.board.get_piece_at_square(capture_square).unwrap();

            self.toggle_accumulators_on(stm.other(), captured_piece, capture_square);
        }

        //Need to toggle rook in case of castling
        if kind == MoveKind::KingCastle {
            debug_assert!(!(from.to_bb() & to_file_bb(Square::E4)).is_empty());
            let king_rook_square = match stm {
                Side::White => KING_SIDE_ROOK_WHITE,
                Side::Black => KING_SIDE_ROOK_BLACK,
            };

            self.toggle_accumulators_on(stm, Piece::Rook, king_rook_square);
            self.toggle_accumulators_off(stm, Piece::Rook, from.shift(1).unwrap());
        }

        //Need to toggle rook in case of castling
        if kind == MoveKind::QueenCastle {
            debug_assert!(!(from.to_bb() & to_file_bb(Square::E4)).is_empty());
            let queen_rook_square = match stm {
                Side::White => QUEEN_SIDE_ROOK_WHITE,
                Side::Black => QUEEN_SIDE_ROOK_BLACK,
            };

            self.toggle_accumulators_on(stm, Piece::Rook, queen_rook_square);
            self.toggle_accumulators_off(stm, Piece::Rook, from.shift(-1).unwrap());
        }

        let moving_piece = self.board.get_piece_at_square(from).unwrap().1;
        //Need to handle promotions
        if kind.is_promotion() {
            let promotion_piece = m.get_promoted_piece().unwrap();
            self.toggle_accumulators_on(stm, moving_piece, from);
            self.toggle_accumulators_off(stm, promotion_piece, to);
        } else {
            self.toggle_accumulators_on(stm, moving_piece, from);
            self.toggle_accumulators_off(stm, moving_piece, to);
        }
    }

    pub fn nnue_evaluate(&self) -> i32 {
        NNUE.evaluate(self)
    }

    pub fn toggle_accumulators_off(&mut self, piece_side: Side, piece: Piece, square: Square) {
        self.white_features
            .toggle_off(piece_side == Side::White, piece, square);
        self.black_features
            .toggle_off(piece_side == Side::Black, piece, square ^ 56);
    }

    pub fn toggle_accumulators_on(&mut self, piece_side: Side, piece: Piece, square: Square) {
        self.white_features
            .toggle_on(piece_side == Side::White, piece, square);
        self.black_features
            .toggle_on(piece_side == Side::Black, piece, square ^ 56);
    }

    pub fn initialize_nnue(&mut self) {
        for rank in 0..8 {
            for file in 0..8 {
                let square = Square::from_rank_and_file(rank, file);
                let side_piece = self.board.get_piece_at_square(square);
                if let Some((side, piece)) = side_piece {
                    self.toggle_accumulators_on(side, piece, square);
                }
            }
        }
    }
}

impl Default for SearchData {
    fn default() -> Self {
        Self::new(Arc::new(SharedData::default()), 0)
    }
}
