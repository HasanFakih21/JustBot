use std::{
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
    thread::JoinHandle,
};

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
    workers: Vec<Worker>,
    shared: Arc<SharedData>,
}

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub board: Board,
    pub root_moves: Vec<RootMove>,
    pub time: TimeManager,
    pub report: Report,
}

impl SearchThreads {
    pub fn new(shared: Arc<SharedData>, count: usize) -> Self {
        let workers = (0..count)
            .map(|id| create_worker(Arc::clone(&shared), id))
            .collect();

        SearchThreads { workers, shared }
    }

    pub fn start(&mut self, board: &Board, mut time: TimeManager, report: Report) -> Option<Move> {
        debug_assert!(!self.workers.is_empty());

        time.set_time_limits(board.state.side_to_move, board.state.full_move);
        self.shared.tt.increase_age();
        self.shared.reset_all_nodes();
        self.shared.status.run();

        let root_moves: Vec<RootMove> = board
            .generate_moves(crate::board::movegen::MoveGenKind::All)
            .iter()
            .map(|e| RootMove {
                m: e.mv,
                ..Default::default()
            })
            .collect();

        let params = SearchParams {
            board: board.clone(),
            root_moves,
            time,
            report,
        };

        for w in self.workers.iter_mut() {
            w.comm
                .send(Command::Search(Box::new(params.clone())))
                .expect("Worker not found");
        }

        let mut result = None;
        for (id, w) in self.workers.iter().enumerate() {
            let r = w.result.recv().expect("Worker not found");
            if id == 0 {
                result = r;
            }
        }

        result
    }

    pub fn count(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for SearchThreads {
    fn drop(&mut self) {
        self.shared.status.stop();
        for w in self.workers.iter() {
            let _ = w.comm.send(Command::Quit);
        }

        self.workers
            .drain(..)
            .for_each(|w| w.handle.join().unwrap());
    }
}

fn create_worker(shared: Arc<SharedData>, id: usize) -> Worker {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    let (result_tx, result_rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        let mut data = SearchData::new(Arc::clone(&shared), id);

        while let Ok(command) = cmd_rx.recv() {
            match command {
                Command::Search(params) => {
                    data.board = params.board;
                    data.root_moves = params.root_moves;
                    data.time = params.time;
                    if id == 0 {
                        data.report = params.report;
                    } else {
                        data.report = Report::None;
                    }

                    search_runner(&mut data);
                    if result_tx.send(data.best_move).is_err() {
                        break;
                    };
                }
                Command::Quit => break,
            }
        }
    });

    Worker {
        handle,
        comm: cmd_tx,
        result: result_rx,
    }
}

enum Command {
    Search(Box<SearchParams>),
    Quit,
}

struct Worker {
    handle: JoinHandle<()>,
    comm: Sender<Command>,
    result: Receiver<Option<Move>>,
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
