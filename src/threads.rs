use std::{
    collections::HashMap,
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
struct SearchParams {
    pub board: Board,
    pub root_moves: Vec<RootMove>,
    pub time: TimeManager,
    pub report: Report,
}

impl SearchThreads {
    pub fn new(shared: Arc<SharedData>, count: usize) -> Self {
        let workers = (0..count).map(|id| create_worker(Arc::clone(&shared), id)).collect();

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

        let mut threads = Vec::new();
        for w in self.workers.iter() {
            let Response::Search(search_result) = w.result.recv().expect("Worker not found") else {
                panic!("Should have recieved a search response here");
            };
            if !search_result.best_move.m.is_null() && search_result.searched_depth > 0 {
                threads.push(search_result);
            }
        }

        let lowest_score = threads.iter().map(|result| result.best_move.score).min().unwrap();
        let mut votes: HashMap<&Move, i32> = HashMap::new();

        for result in threads.iter() {
            *votes.entry(&result.best_move.m).or_default() +=
                (result.best_move.score - lowest_score + 10) * result.searched_depth;
        }

        let mut best_index = 0;

        for current_index in 0..threads.len() {
            let best = &threads[best_index].best_move;
            let current = &threads[current_index].best_move;

            if votes[&best.m] > votes[&current.m] {
                best_index = current_index;
            }
        }

        if report != Report::None {
            self.workers[threads[best_index].id]
                .comm
                .send(Command::PrintUCI)
                .expect("Worker {id} was supposed to print uci but couldn't");

            let Response::PrintUCI = self.workers[threads[best_index].id]
                .result
                .recv()
                .expect("Printing worker didn't respond!")
            else {
                unreachable!();
            };
        }

        Some(threads[best_index].best_move.m)
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

        for w in self.workers.drain(..) {
            let _ = w.handle.join();
        }
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
                    if result_tx
                        .send(Response::Search(SearchResult {
                            id: data.id,
                            best_move: data.best_move.clone().unwrap_or_default(),
                            searched_depth: data.completed_depth,
                        }))
                        .is_err()
                    {
                        break;
                    };
                }
                Command::Quit => break,
                Command::PrintUCI => {
                    data.print_uci_info();
                    if result_tx.send(Response::PrintUCI).is_err() {
                        break;
                    }
                }
            }
        }
    });

    Worker {
        handle,
        comm: cmd_tx,
        result: result_rx,
    }
}

struct SearchResult {
    id: usize,
    best_move: RootMove,
    searched_depth: i32,
}

enum Response {
    Search(SearchResult),
    PrintUCI,
}

enum Command {
    Search(Box<SearchParams>),
    PrintUCI,
    Quit,
}

struct Worker {
    handle: JoinHandle<()>,
    comm: Sender<Command>,
    result: Receiver<Response>,
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
