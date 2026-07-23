use std::cmp::max;

use crate::board::Board;
use crate::types::{BitBoard, Piece, Square};

impl Board {
    //Only checks for the current side to move
    pub fn only_king_and_pawns(&self) -> bool {
        let side = self.state.side_to_move;
        self.get_piece_bb(side, Piece::Bishop)
            | self.get_piece_bb(side, Piece::Knight)
            | self.get_piece_bb(side, Piece::Queen)
            | self.get_piece_bb(side, Piece::Rook)
            == BitBoard(0)
    }

    //Needs fixing
    pub fn detect_repetitions(&self) -> usize {
        let half_moves = self.state.half_move_clock as usize;
        let mut count = 0;

        if self.state_stack.len() < half_moves {
            return 0;
        }

        let last_halfmove_ply = self.state_stack.len() - half_moves;
        for position in self.state_stack[last_halfmove_ply..].iter() {
            if self.state.hash == position.hash {
                count += 1
            }
        }

        count
    }
}

//https://www.chessprogramming.org/Center_Manhattan-Distance
pub const fn cmd(square: Square) -> usize {
    let (mut file, mut rank) = square.to_rank_and_file();

    file ^= (file.wrapping_sub(4)) >> 8;
    rank ^= (rank.wrapping_sub(4)) >> 8;

    file.wrapping_add(rank) & 7
}

pub const fn manhattan_distance(square_1: Square, square_2: Square) -> usize {
    let (rank1, file1) = square_1.to_rank_and_file();
    let (rank2, file2) = square_2.to_rank_and_file();

    let rank_distance = rank2.abs_diff(rank1);
    let file_distance = file2.abs_diff(file1);

    rank_distance + file_distance
}

//Chebyshev Distance
pub fn distance(square_1: Square, square_2: Square) -> usize {
    let (rank1, file1) = square_1.to_rank_and_file();
    let (rank2, file2) = square_2.to_rank_and_file();

    let rank_distance = rank2.abs_diff(rank1);
    let file_distance = file2.abs_diff(file1);

    max(rank_distance, file_distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd() {
        let square = Square::A8;

        for rank in 0..8 {
            for file in 0..8 {
                let square = Square::from_rank_and_file(rank, file);
                print!("{} ", cmd(square));
            }
            println!()
        }

        assert_eq!(6, cmd(square));
    }

    #[test]
    fn test_distances() {
        let square_1 = Square::A8;
        let square_2 = Square::A4;

        assert_eq!(4, distance(square_1, square_2));

        let square_1 = Square::B4;
        let square_2 = Square::A4;

        assert_eq!(1, distance(square_1, square_2));

        let square_1 = Square::H8;
        let square_2 = Square::A1;

        assert_eq!(7, distance(square_1, square_2));
        assert_eq!(14, manhattan_distance(square_1, square_2));
    }
}
