// Converting evals with NormalizeToPawnValue = 100.
// Reading eval stats from scoreWDLstat.json.
// Retained (W,D,L) = (120979, 515460, 127556) positions.
// Fit WDL model based on material.
// Initial objective function:  0.4813508737538977
// Final objective function:    0.48132996721493493
// Optimization terminated successfully.
// const int NormalizeToPawnValue = 299;
// Corresponding spread = 82;
// Corresponding normalized spread = 0.27333365708192797;
// Draw rate at 0.0 eval at material 58 = 0.9497542304215496;
// Parameters in internal value units:
// p_a = ((-41.503 * x / 58 + 8.757) * x / 58 + 19.237) * x / 58 + 312.568
// p_b = ((-57.571 * x / 58 + 155.986) * x / 58 + -76.702) * x / 58 + 60.030
//     constexpr double as[] = {-41.50318996, 8.75690237, 19.23737818, 312.56845419};
//     constexpr double bs[] = {-57.57065708, 155.98638684, -76.70226158, 60.02957087};

use crate::{
    board::Board,
    types::{Piece, is_decisive},
};

fn wdl_params(material: i32) -> (f64, f64) {
    const A: [f64; 4] = [-41.50318996, 8.75690237, 19.23737818, 312.56845419];
    const B: [f64; 4] = [-57.57065708, 155.98638684, -76.70226158, 60.02957087];

    let m = material.clamp(17, 78) as f64 / 58.0;

    (
        ((A[0] * m + A[1]) * m + A[2]) * m + A[3],
        ((B[0] * m + B[1]) * m + B[2]) * m + B[3],
    )
}

fn material(board: &Board) -> i32 {
    (1 * board.state.pieces[Piece::Pawn].count_bits()
        + 3 * board.state.pieces[Piece::Knight].count_bits()
        + 3 * board.state.pieces[Piece::Bishop].count_bits()
        + 5 * board.state.pieces[Piece::Rook].count_bits()
        + 9 * board.state.pieces[Piece::Queen].count_bits()) as i32
}

pub fn wdl_model(score: i32, board: &Board) -> (i32, i32) {
    let material = material(board);
    let (a, b) = wdl_params(material);
    let x = score as f64;

    (
        (1000.0 / (1.0 + ((a - x) / b).exp())) as i32,
        (1000.0 / (1.0 + ((a - x) / b).exp())) as i32,
    )
}

pub fn normalize_score(score: i32, board: &Board) -> i32 {
    if score == 0 || is_decisive(score) {
        return score;
    }

    let material = material(board);
    let (a, _) = wdl_params(material);

    (score as f64 / a * 100.0).round() as i32
}
