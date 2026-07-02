use std::{collections::HashMap, sync::Arc};

use crate::{
    board::Board,
    search::{
        data::{SearchData, SharedData},
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
        for _ in 0..count {
            threads.push(SearchData::new(shared.clone()));
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
        shared.clear_node_count();
        shared.tt.increase_age();
        time.set_time_limits(board.state.side_to_move);

        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for (i, t) in self.threads.iter_mut().enumerate() {
                if i != 0 || mute {
                    t.mute();
                }

                t.board = board.clone();
                t.time = time.clone();

                handles.push(scope.spawn(|| search_runner(t)));
            }

            let mut best_moves = HashMap::new();
            for handle in handles {
                if let Ok(Some(m)) = handle.join() {
                    best_moves.insert(m.mv, best_moves.get(&m.mv).unwrap_or(&0) + 1);
                }
            }

            best_moves
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(m, _)| m)
        })
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
