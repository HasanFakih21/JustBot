use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;

use crate::board::Board;
use crate::search::data::SharedData;
use crate::search::time::TimeManager;
use crate::threads::SearchThreads;
use crate::tools::bench::bench;
use crate::tools::datagen::generate_random_openings;
use crate::types::*;

pub fn input_loop(cli_args: String) {
    let shared = Arc::new(SharedData::default());
    let mut pool = SearchThreads::new(shared.clone(), 1);
    let mut board = Board::from_fen(STARTING_FEN).unwrap();
    let mut time = TimeManager::new();

    let rx = listen(shared.clone());

    let mut input = if !cli_args.is_empty() {
        cli_args
    } else {
        String::new()
    };

    loop {
        if input.is_empty() {
            if let Ok(s) = rx.recv() {
                input = s;
            } else {
                shared.status.stop();
                break;
            }
        }

        let (command, args) = input.split_once(" ").unwrap_or((&input, ""));

        match command.trim() {
            "position" => position(args, &mut board),
            "uci" => uci(),
            "isready" => println!("readyok"),
            "setoption" => set_option(args, shared.clone(), &mut pool),
            "ucinewgame" => {
                shared.tt.clear();
                let thread_count = pool.threads.len();
                pool = SearchThreads::new(shared.clone(), thread_count);
            }
            "go" => {
                time.clear_limits();
                shared.status.run();

                if let Some(m) = go(args, &mut pool, &mut board, &mut time, &shared, false) {
                    println!("bestmove {}", m);
                } else {
                    println!("bestmove 0000");
                }
            }
            "quit" => break,
            "perft" => {
                if let Ok(depth) = args.trim().parse::<usize>() {
                    crate::tools::perft::perft(depth, &mut board);
                } else {
                    eprintln!("Invalid depth: {:?}", args);
                }
            }
            "d" => println!("{}", board),
            "bench" => {
                let (total_node_count, nps) = bench();
                println!("{} nodes {} nps", total_node_count, nps);
                break;
            }
            "genfens" => {
                genfens(args);
            }
            _ => (),
        }

        if input.contains("quit") {
            break;
        }

        input.clear();
    }
}

pub fn listen(shared: Arc<SharedData>) -> Receiver<String> {
    let (tx, rx) = channel::<String>();
    let mut input_buffer = String::new();

    thread::spawn(move || {
        loop {
            if std::io::stdin().read_line(&mut input_buffer).unwrap() == 0 {
                shared.status.stop();
                break;
            };

            match input_buffer.trim() {
                "quit" => {
                    shared.status.stop();
                    break;
                }
                "stop" => {
                    shared.status.stop();
                }
                _ => (),
            }

            let _ = tx.send(input_buffer.clone());
            input_buffer.clear();
        }
    });

    rx
}

pub fn position(args: &str, board: &mut Board) {
    if args.trim().is_empty() {
        eprintln!("Need to provide a valid argument!");
        return;
    }

    let (command, args) = args.split_once(" ").unwrap_or((args, ""));
    let (args, moves) = args.split_once("moves").unwrap_or((args, ""));

    match command.trim() {
        "startpos" => {
            *board = Board::from_fen(STARTING_FEN).unwrap();
        }
        "fen" => {
            if args.trim().is_empty() {
                eprintln!("Please provide a fen string");
                return;
            }
            if let Ok(b) = Board::from_fen(args) {
                *board = b;
            } else {
                eprintln!("Invalid FEN: {:?}", args.trim_end());
            }
        }
        _ => eprintln!("Not a valid position argument!"),
    }

    if !moves.trim().is_empty() {
        for m_str in moves.split_ascii_whitespace() {
            let result = board.parse_move(m_str);
            if let Ok(m) = result {
                board.make_move(m);
            } else {
                eprintln!("Illegal Move!");
                return;
            }
        }
    }
}

pub fn set_option(args: &str, shared: Arc<SharedData>, pool: &mut SearchThreads) {
    let args = args.to_ascii_lowercase();
    let args: Vec<&str> = args.split_ascii_whitespace().collect();
    match args.as_slice() {
        ["name", "hash", "value", amount] => {
            let amount = amount.parse::<usize>().unwrap_or(16);
            shared.tt.resize(amount);
            println!("info string Resized TT to {amount} mb");
        }
        ["name", "threads", "value", amount] => {
            let amount = amount.parse::<usize>().unwrap_or(1);
            *pool = SearchThreads::new(shared, amount);
        }
        ["name", "clear", "hash"] => {
            shared.tt.clear();
            println!("info string TT cleared");
        }
        _ => eprintln!("Unkown option"),
    }
}

pub fn go(
    args: &str,
    pool: &mut SearchThreads,
    board: &mut Board,
    time: &mut TimeManager,
    shared: &Arc<SharedData>,
    mute: bool,
) -> Option<Move> {
    let (command, args) = args.split_once(" ").unwrap_or((args, ""));
    if args.is_empty() {
        return pool.start(board, time.clone(), shared, mute);
    }

    match command.trim() {
        "depth" => {
            let (depth, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.depth = depth.trim().parse().unwrap_or(MAX_PLY as i32 - 1);
            time.set_depth_limit();
            go(args, pool, board, time, shared, mute)
        }
        "wtime" => {
            //Example: go wtime 900000 btime 900000 winc 0 binc 0
            let (wtime, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.wtime = Some(wtime.trim().parse().unwrap_or(500));
            go(args, pool, board, time, shared, mute)
        }
        "btime" => {
            let (btime, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.btime = Some(btime.trim().parse().unwrap_or(500));
            go(args, pool, board, time, shared, mute)
        }
        "winc" => {
            let (winc, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.winc = winc.trim().parse().unwrap_or(0);
            go(args, pool, board, time, shared, mute)
        }
        "binc" => {
            let (binc, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.binc = binc.trim().parse().unwrap_or(0);
            go(args, pool, board, time, shared, mute)
        }
        "movestogo" => {
            let (movestogo, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.movestogo = movestogo.trim().parse().unwrap_or(0);
            go(args, pool, board, time, shared, mute)
        }
        "movetime" => {
            let (movetime, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.movetime = movetime.trim().parse().unwrap_or(0);
            go(args, pool, board, time, shared, mute)
        }
        "nodes" => {
            let (nodes, args) = args.split_once(" ").unwrap_or((args, ""));
            time.settings.nodes = nodes.trim().parse().unwrap_or(0);
            time.set_nodes_limit();
            go(args, pool, board, time, shared, mute)
        }
        _ => go(args, pool, board, time, shared, mute),
    }
}

pub fn uci() {
    println!("id name JustBot {}", env!("CARGO_PKG_VERSION"));
    println!("id author Hasan Fakih");
    println!("option name Threads type spin default 1 min 1 max 512");
    println!("option name Hash type spin default 16 min 1 max 1048576");
    println!("option name Clear Hash type button");
    println!("uciok");
}

pub fn genfens(args: &str) {
    let args = args.to_ascii_lowercase();
    let args: Vec<&str> = args.split_ascii_whitespace().collect();
    let mut amount = 0;
    let mut seed = 0;

    match args.as_slice() {
        [n, "seed", s, ..] => {
            amount = n.parse::<usize>().unwrap_or(0);
            seed = s.parse::<u64>().unwrap_or(0);
        }
        [n, ..] => {
            amount = n.parse::<usize>().unwrap_or(0);
        }
        _ => (),
    }

    generate_random_openings(amount, 8, seed);
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use crate::types::constants::STARTING_FEN;

    #[test]
    fn test_parse_move() {
        let board = Board::from_fen(STARTING_FEN).unwrap();
        if let Ok(m) = board.parse_move("e2e4") {
            println!("bestmove {m}");
        }
    }

    #[test]
    fn test_parse_times() {
        let shared = Arc::new(SharedData::default());
        let mut pool = SearchThreads::new(shared.clone(), 1);
        let mut board = Board::from_fen(STARTING_FEN).unwrap();
        let mut time = TimeManager::new();

        go(
            "wtime 5000 btime 5000 winc 0 binc 0",
            &mut pool,
            &mut board,
            &mut time,
            &shared,
            false,
        );
    }

    #[test]
    fn test_parse_go() {
        let shared = Arc::new(SharedData::default());
        let mut pool = SearchThreads::new(shared.clone(), 1);
        let mut board = Board::from_fen(STARTING_FEN).unwrap();
        let mut time = TimeManager::new();
        let bm = go(
            "wtime 5000 btime 5000 winc 5 binc 8 movetime 100",
            &mut pool,
            &mut board,
            &mut time,
            &shared,
            false,
        );
        println!("{:?}\nBestmove: {}", time.settings, bm.unwrap());
    }

    #[test]
    fn test_set_option() {
        let shared = Arc::new(SharedData::default());
        let mut pool = SearchThreads::new(shared.clone(), 1);

        set_option("name Hash value 32", shared, &mut pool);
    }
}
