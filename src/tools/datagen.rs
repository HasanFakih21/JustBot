use std::{
    error::Error,
    fmt::Display,
    fs::File,
    io::{self, BufRead, BufReader},
};

use rand::{RngExt, SeedableRng, random_range, rngs::StdRng};

use crate::{
    board::{Board, movegen::MoveGenKind, parser::FenParseError},
    search::data::SearchData,
    types::STARTING_FEN,
};

pub fn begin_genfens(amount: usize, seed: u64, book: Option<File>) -> io::Result<()> {
    let (lines, plies) = if let Some(file) = book {
        (BufReader::new(file).lines().collect::<io::Result<_>>()?, 5)
    } else {
        (vec![STARTING_FEN.to_string()], 8)
    };

    let mut rng = StdRng::seed_from_u64(seed);

    for _ in 0..amount {
        let mut random_board = generate_random_opening(plies, &mut rng, &lines);

        // Regenerate imbalanced positions
        while random_board.is_err() {
            random_board = generate_random_opening(plies, &mut rng, &lines);
        }

        println!("info string genfens {}", random_board.unwrap().to_fen());
    }

    Ok(())
}

pub fn generate_random_opening(plies: isize, rng: &mut StdRng, book: &[String]) -> Result<Board, BadRandomBoard> {
    let mut data = SearchData::default();
    data.network.full_refresh(&data.board);
    data.board = Board::from_fen(&book[random_range(0..book.len())])?;

    let plies = if rng.random_bool(0.5) { plies } else { plies + 1 };

    for ply in 0..plies {
        let move_list = data.board.generate_moves(MoveGenKind::All);
        // Check if there's atleast one legal move first
        if move_list.is_empty() {
            return Err(BadRandomBoard);
        }

        let index = rng.random_range(0..move_list.len());
        let random_move = move_list.get(index).mv;
        data.make_move(random_move, ply);
    }

    // Check if eval is not too uneven
    if data.network.evaluate(&data.board).abs() > 1000 {
        return Err(BadRandomBoard);
    }

    Ok(data.board)
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
