use std::sync::Mutex;

use crate::{
    board::{Board, movegen::MoveGenKind},
    search::data::SearchData,
    tools::bench::bench,
    types::pseudo_rand,
};

#[derive(Debug)]
pub struct BadRandomBoard;

static SEED: Mutex<u64> = Mutex::new(0);

pub fn generate_random_openings(amount: usize, plies: isize, seed: u64) -> Vec<String> {
    if seed != 0 {
        *SEED.lock().unwrap() = seed;
    } else {
        *SEED.lock().unwrap() = bench().1;
    }

    let mut openings = Vec::new();
    for _ in 0..amount {
        let mut random_number = pseudo_rand(&mut SEED.lock().unwrap());
        let mut random_board = randomize_from_startpos(plies, random_number);

        //Regenerate imbalanced positions
        while random_board.is_err() {
            random_number = pseudo_rand(&mut SEED.lock().unwrap());
            random_board = randomize_from_startpos(plies, random_number);
        }

        openings.push(random_board.unwrap().to_fen());
    }

    openings
}

pub fn randomize_from_startpos(plies: isize, random_number: u64) -> Result<Board, BadRandomBoard> {
    let mut data = SearchData::default();
    data.initialize_nnue();

    let mut state = random_number;
    for ply in 0..plies {
        let move_list = data.board.generate_moves(MoveGenKind::All);
        //Check if there's atleast one legal move first
        if move_list.is_empty() {
            return Err(BadRandomBoard);
        }

        let index = pseudo_rand(&mut state) % move_list.len() as u64;
        let random_move = move_list.get(index as usize).mv;
        data.make_move(random_move, ply);
    }

    //Check if eval is not too uneven
    if data.nnue_evaluate().abs() > 1000 {
        return Err(BadRandomBoard);
    }

    Ok(data.board)
}

#[cfg(test)]
pub mod tests {
    use crate::tools::datagen::generate_random_openings;

    #[test]
    fn test_fengen() {
        let book = generate_random_openings(1, 8, 3493);
        for opening in book {
            println!("{opening}");
        }
    }
}
