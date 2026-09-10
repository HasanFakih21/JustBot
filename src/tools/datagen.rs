use std::{
    error::Error,
    fmt::Display,
    fs::File,
    io::{self, BufRead, BufReader},
};

use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    board::{Board, movegen::MoveGenKind, parser::FenParseError},
    search::{
        data::{RootMove, SearchData},
        time::{Limit, NodeKind, TimeManager},
    },
    types::STARTING_FEN,
};

pub fn begin_genfens(amount: usize, seed: u64, book: Option<File>) -> io::Result<()> {
    let lines = if let Some(file) = book {
        BufReader::new(file).lines().collect::<io::Result<_>>()?
    } else {
        Vec::new()
    };

    assert!(
        !lines.iter().any(|fen| Board::from_fen(fen).is_err()),
        "info error: illegal fen entered"
    );
    println!("info string found {} book lines", lines.len());
    let plies = if lines.is_empty() { 7 } else { 5 };

    let mut rng = StdRng::seed_from_u64(seed);
    let mut data = SearchData::default();

    for _ in 0..amount {
        let mut random_board = generate_random_opening(&mut data, plies, &mut rng, &lines);

        // Regenerate imbalanced positions
        while random_board.is_err() {
            random_board = generate_random_opening(&mut data, plies, &mut rng, &lines);
        }

        println!("info string genfens {}", random_board.unwrap());
    }

    Ok(())
}

fn generate_random_opening(
    data: &mut SearchData,
    plies: isize,
    rng: &mut StdRng,
    book: &[String],
) -> Result<String, BadRandomBoard> {
    let fen = if book.is_empty() { STARTING_FEN } else { &book[rng.random_range(0..book.len())] };
    data.board = Board::from_fen(fen)?;

    let plies = if rng.random_bool(0.5) { plies } else { plies + 1 };

    for _ in 0..plies {
        let move_list = data.board.generate_moves(MoveGenKind::All);
        // Check if there's atleast one legal move first
        if move_list.is_empty() {
            return Err(BadRandomBoard);
        }

        let index = rng.random_range(0..move_list.len());
        let random_move = move_list.get(index).mv;
        data.board.make_move(random_move);
    }

    // Check if eval is not too uneven
    validation_search(data, Limit::Nodes(NodeKind::Soft(20_000)));
    let Some(best_move) = data.best_move.as_ref() else { return Err(BadRandomBoard) };

    if best_move.score.abs() > 400 || best_move.score.abs() < 25 {
        return Err(BadRandomBoard);
    }

    Ok(data.board.to_fen())
}

fn validation_search(data: &mut SearchData, limit: Limit) {
    data.time = TimeManager::new(limit, 0);
    data.shared.reset_all_nodes();
    data.shared.status.run();
    data.root_moves = data
        .board
        .generate_moves(MoveGenKind::All)
        .iter()
        .map(|e| RootMove {
            m: e.mv,
            ..Default::default()
        })
        .collect();

    crate::search::search_runner(data);
}

#[derive(Debug)]
pub struct BadRandomBoard;

impl Error for BadRandomBoard {}

impl From<FenParseError> for BadRandomBoard {
    fn from(_: FenParseError) -> Self {
        Self
    }
}

impl Display for BadRandomBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad random board")
    }
}
