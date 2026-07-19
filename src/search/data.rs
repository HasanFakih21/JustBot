use std::array;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::board::Board;
use crate::nnue::{Accumulator, NNUE};
use crate::search::time::{TimeManager, TimeSettings};
use crate::types::plytable::PlyTable;
use crate::types::pv::PVTable;
use crate::types::{
    ContinuationHistory, Move, NoisyHistory,
    Piece, STARTING_FEN, Score, Side, Square,
    is_mate,
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
    pub status: Status,
    pub nodes: Box<[AtomicU64; 512]>,
}

impl SharedData {
    pub fn increment_nodes(&self, id: usize) {
        self.nodes[id].store(
            self.nodes[id].load(Ordering::Relaxed) + 1,
            Ordering::Relaxed,
        );
    }

    pub fn get_node_count(&self, id: usize) -> u64 {
        self.nodes[id].load(Ordering::Relaxed)
    }

    pub fn get_total_nodes_searched(&self) -> u64 {
        self.nodes.iter().map(|n| n.load(Ordering::Relaxed)).sum()
    }

    pub fn reset_all_nodes(&self) {
        for t in self.nodes.iter() {
            t.store(0, Ordering::Relaxed);
        }
    }
}

impl Default for SharedData {
    fn default() -> Self {
        Self {
            tt: TranspositionTable::default(),
            status: Status(AtomicBool::new(Status::RUNNING)),
            nodes: Box::new(array::from_fn(|_| AtomicU64::new(0))),
        }
    }
}

#[derive(Debug)]
pub struct SearchData {
    pub id: usize,
    pub best_move: Option<Move>,
    pub shared: Arc<SharedData>,
    pub pv: PVTable,
    pub board: Board,
    pub time: TimeManager,
    pub report: bool,
    pub ply_table: Box<PlyTable>,

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
            pv: PVTable::new(),
            board: Board::from_fen(STARTING_FEN).unwrap(),
            time: TimeManager::new(),
            ply_table: PlyTable::new(),

            quiet_history: QuietHistory::new(),
            noisy_history: NoisyHistory::new(),
            conthistory: ContinuationHistory::new(),
            report: true,

            white_features: Accumulator::new(&NNUE),
            black_features: Accumulator::new(&NNUE),
        }
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

    pub fn start_time(&mut self) {
        self.time.reset_clock();
    }

    pub fn nodes(&self) -> u64 {
        self.shared.get_node_count(self.id)
    }

    pub fn reset_nodes(&self) {
        self.shared.nodes[self.id].store(0, Ordering::Relaxed);
    }

    pub fn nodes_per_second(&self) -> usize {
        (self.shared.get_total_nodes_searched() as f32 / self.time.elapsed().as_secs_f32()) as usize
    }

    pub fn get_time_settings(&mut self) -> &mut TimeSettings {
        &mut self.time.settings
    }

    pub fn reset_pv(&mut self) {
        self.pv = PVTable::new();
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
            let score_print = if is_mate(score) {
                let num_plies = Score::MATE - score.abs();
                let mate_in = score.signum() * ((num_plies + 1) / 2);
                format!("mate {}", mate_in)
            } else {
                format!("cp {}", score)
            };

            let pv_display = {
                let mut output = String::new();
                for m in self.pv.line() {
                    output = format!("{output}{m} ");
                }

                output
            };

            println!(
                "info depth {} time {} score {} nodes {} nps {} pv {} hashfull {}",
                depth - 1,
                self.time.elapsed().as_millis(),
                score_print,
                self.shared.get_total_nodes_searched(),
                self.nodes_per_second(),
                pv_display,
                self.shared.tt.hashfull(),
            );
        }
    }

    //Called before move is made on the board
    pub fn make_move(&mut self, m: Move, ply: isize) {
        let stm = self.board.state.side_to_move;
        let moving_piece = self.board.get_piece_at_square(m.get_from()).unwrap().1;

        self.ply_table[ply].m = m;
        self.ply_table[ply].piece = Some((stm, moving_piece));
        self.ply_table[ply].conthistory = self
            .conthistory
            .subtable(Some((stm, moving_piece)), m.get_to());

        self.board.make_move(m)
    }

    //Called after move is already unmade on the board
    pub fn unmake_move(&mut self) {
        self.board.unmake_move();
    }

    pub fn nnue_evaluate(&mut self) -> i32 {
        self.clear_features();
        self.initialize_nnue();

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
