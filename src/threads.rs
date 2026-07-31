use std::sync::Arc;

use crate::{
    board::Board,
    search::{
        data::{RootMove, SearchData, SharedData},
        search_runner,
        time::TimeManager,
    },
    types::Move,
};

pub struct SearchThreads {
    pub threads: Vec<SearchData>,
}

impl SearchThreads {
    pub fn new(shared: std::sync::Arc<SharedData>, count: usize) -> Self {
        let mut threads = Vec::new();
        for id in 0..count {
            threads.push(SearchData::new(shared.clone(), id));
        }

        SearchThreads { threads }
    }

    pub fn start(
        &mut self,
        board: &Board,
        mut time: TimeManager,
        shared: &Arc<SharedData>,
        mute: bool,
    ) -> Option<Move> {
        shared.tt.increase_age();
        time.set_time_limits(board.state.side_to_move);
        shared.reset_all_nodes();
        let root_moves: Vec<RootMove> = board
            .generate_moves(crate::board::movegen::MoveGenKind::All)
            .iter()
            .map(|e| RootMove {
                m: e.mv,
                nodes: 0,
                score: 0,
            })
            .collect();

        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for t in self.threads.iter_mut() {
                if t.id != 0 || mute {
                    t.mute();
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

        self.threads.first().map(|t| t.best_move)?
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        board::Board,
        search::{data::SharedData, time::TimeManager},
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
        let m = pool.start(&board, time, &shared, false).unwrap();
        println!("{}", m);
    }
}
