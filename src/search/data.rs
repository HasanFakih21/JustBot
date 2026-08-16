use std::array;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::board::Board;
use crate::nnue::Network;
use crate::search::time::{TimeManager, TimeSettings};
use crate::types::pv::PVTable;
use crate::types::stack::Stack;
use crate::types::{
    ContinuationCorrectionHistory, ContinuationHistory, CorrectionHistory, Move, NoisyHistory,
    PawnHistory, STARTING_FEN, Score, Side, is_decisive,
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

pub struct SearchData {
    pub id: usize,
    pub best_move: Option<Move>,
    pub shared: Arc<SharedData>,
    pub pv: PVTable,
    pub board: Board,
    pub time: TimeManager,
    pub report: bool,
    pub stack: Box<Stack>,
    pub root_moves: Vec<RootMove>,

    pub quiet_history: QuietHistory,
    pub noisy_history: NoisyHistory,
    pub pawn_history: PawnHistory,
    pub conthistory: ContinuationHistory,
    pub corrhistory: CorrectionHistories,
    pub contcorrhistory: ContinuationCorrectionHistory,

    pub network: Network,
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
            report: true,
            stack: Stack::new(),
            root_moves: Vec::new(),

            quiet_history: QuietHistory::new(),
            noisy_history: NoisyHistory::new(),
            pawn_history: PawnHistory::new(),
            conthistory: ContinuationHistory::new(),
            corrhistory: CorrectionHistories::default(),
            contcorrhistory: ContinuationCorrectionHistory::new(),

            network: Network::new(),
        }
    }

    pub fn mute(&mut self) {
        self.report = false;
    }

    pub fn report(&mut self) {
        self.report = true;
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
            for i in [1, 2, 4] {
                self.conthistory.update(
                    self.stack[ply - i].conthistory,
                    self.board.get_piece_at_square(m.get_from()),
                    m.get_to(),
                    bonus,
                );
            }
        }
    }

    pub fn update_correction_histories(&mut self, diff: i32, depth: i32, ply: isize) {
        let stm = self.board.state.side_to_move;
        let bonus = (148 * depth * diff / 121).clamp(-4612, 2530);
        self.corrhistory
            .pawn
            .update(stm, self.board.state.keys.pawn, bonus);
        self.corrhistory.non_pawn[Side::White as usize].update(
            stm,
            self.board.state.keys.non_pawn[Side::White],
            bonus,
        );
        self.corrhistory.non_pawn[Side::Black as usize].update(
            stm,
            self.board.state.keys.non_pawn[Side::Black],
            bonus,
        );

        unsafe {
            if !self.stack[ply - 1].m.is_null() && !self.stack[ply - 2].m.is_null() {
                self.contcorrhistory.update(
                    self.stack[ply - 2].contcorrhistory,
                    self.stack[ply - 1].piece,
                    self.stack[ply - 1].m.get_to(),
                    bonus,
                );
            }
        }
    }

    pub fn correction(&self, ply: isize) -> i32 {
        let stm = self.board.state.side_to_move;
        (self.corrhistory.pawn.get(stm, self.board.state.keys.pawn)
            + self.corrhistory.non_pawn[Side::White as usize]
                .get(stm, self.board.state.keys.non_pawn[Side::White])
            + self.corrhistory.non_pawn[Side::Black as usize]
                .get(stm, self.board.state.keys.non_pawn[Side::Black])
            + unsafe {
                if !self.stack[ply - 1].m.is_null() && !self.stack[ply - 2].m.is_null() {
                    self.contcorrhistory.get(
                        self.stack[ply - 2].contcorrhistory,
                        self.stack[ply - 1].piece,
                        self.stack[ply - 1].m.get_to(),
                    )
                } else {
                    0
                }
            })
            / 64
    }

    pub fn get_conthistory(&self, m: Move, ply: isize, index: isize) -> i32 {
        unsafe {
            self.conthistory.get(
                self.stack[ply - index].conthistory,
                self.board.get_piece_at_square(m.get_from()),
                m.get_to(),
            )
        }
    }

    pub fn print_uci_info(&self, score: i32, depth: i32, board: &Board) {
        // All infos belonging to the pv should be sent together e.g. info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3
        if self.report {
            // Report mate score
            let score_print = if is_decisive(score) {
                let num_plies = Score::MATE - score.abs();
                let mate_in = score.signum() * ((num_plies + 1) / 2);
                format!("mate {}", mate_in)
            } else {
                format!("cp {}", score)
            };

            let pv_display = {
                let mut output = String::new();
                for m in self.pv.line() {
                    output = format!("{output}{} ", m.to_uci(board));
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

    pub fn make_move(&mut self, m: Move, ply: isize) {
        self.network.push(&self.board, m);

        let from = m.get_from();
        let to = m.get_to();
        let piece = self.board.get_piece_at_square(from);

        self.stack[ply].m = m;
        self.stack[ply].piece = piece;
        self.stack[ply].conthistory = self.conthistory.subtable(piece, to);
        self.stack[ply].contcorrhistory = self.contcorrhistory.subtable(piece, to);

        self.board.make_move(m);
    }

    pub fn unmake_move(&mut self) {
        self.board.unmake_move();
        self.network.pop();
    }
}

impl Default for SearchData {
    fn default() -> Self {
        Self::new(Arc::new(SharedData::default()), 0)
    }
}

#[derive(Debug, Default)]
pub struct CorrectionHistories {
    pub pawn: CorrectionHistory,
    pub non_pawn: [CorrectionHistory; 2],
}

#[derive(Debug, Default, Clone)]
pub struct RootMove {
    pub m: Move,
    pub score: i32,
    pub nodes: u64,
}
