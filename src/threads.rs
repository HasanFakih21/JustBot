use crate::{
    board::Board,
    search::{
        data::{Report, RootMove, SearchData, SharedData},
        search_runner,
        time::TimeManager,
    },
    types::Move,
};

pub struct SearchThreads {
    threads: Vec<SearchData>,
}

impl SearchThreads {
    pub fn new(shared: std::sync::Arc<SharedData>, count: usize) -> Self {
        let mut threads = Vec::new();
        for id in 0..count {
            threads.push(SearchData::new(shared.clone(), id));
        }

        SearchThreads { threads }
    }

    pub fn start(&mut self, board: &Board, mut time: TimeManager, report: Report) -> Option<Move> {
        debug_assert!(!self.threads.is_empty());

        time.set_time_limits(board.state.side_to_move, board.state.full_move);
        self.threads[0].shared.tt.increase_age();
        self.threads[0].shared.reset_all_nodes();
        self.threads[0].shared.status.run();

        let root_moves: Vec<RootMove> = board
            .generate_moves(crate::board::movegen::MoveGenKind::All)
            .iter()
            .map(|e| RootMove {
                m: e.mv,
                ..Default::default()
            })
            .collect();

        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for t in self.threads.iter_mut() {
                if t.id == 0 {
                    t.report = report;
                }

                t.board = board.clone();
                t.time = time.clone();
                t.root_moves = root_moves.clone();

                handles.push(scope.spawn(|| search_runner(t)));
            }

            for handle in handles {
                let _ = handle.join();
            }
        });

        self.threads[0].best_move
    }

    pub fn count(&self) -> usize {
        self.threads.len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        board::Board,
        search::{
            data::{Report, SharedData},
            time::TimeManager,
        },
        threads::SearchThreads,
        types::STARTING_FEN,
    };

    #[test]
    fn test_multithread() {
        let a = std::thread::available_parallelism().unwrap().get();
        println!("{a}");

        let shared = Arc::new(SharedData::default());
        let mut time = TimeManager::new();
        time.settings.wtime = Some(8080);
        time.settings.winc = 80;

        let board = Board::from_fen(STARTING_FEN).unwrap();

        let mut pool = SearchThreads::new(shared.clone(), 3);
        let m = pool.start(&board, time, Report::None).unwrap();
        println!("{}", m.to_uci(&board));
    }
}
