use crate::board::{Board, movegen::MoveGenKind};

pub fn perft(depth: usize, board: &mut Board) -> usize {
    let clock = std::time::Instant::now();
    let mut nodes_count = 0;

    for m in board.generate_moves(MoveGenKind::All).iter() {
        debug_assert!(
            board.is_legal(m.mv),
            "Tried making move: {}\n{}",
            m.mv.to_uci(board),
            board
        );

        board.make_move(m.mv);
        let divided_nodes = perft_divide(depth - 1, board);
        println!("{}: {divided_nodes}", m.mv.to_uci(board));
        nodes_count += divided_nodes;
        board.unmake_move();
    }

    println!(
        "Number of nodes: {nodes_count}\nTime: {}ms\nNPS: {}",
        clock.elapsed().as_millis(),
        (nodes_count as f64 / clock.elapsed().as_secs_f64()) as usize
    );

    nodes_count
}

pub fn perft_divide(depth: usize, board: &mut Board) -> usize {
    if depth == 0 {
        return 1;
    }

    if depth == 1 {
        return board.generate_moves(MoveGenKind::All).len();
    }

    let mut nodes = 0;

    for m in board.generate_moves(MoveGenKind::All).iter() {
        board.make_move(m.mv);
        nodes += perft_divide(depth - 1, board);
        board.unmake_move()
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::STARTING_FEN;

    macro_rules! assert_perft {
        ($($fen:expr, [$($nodes:expr), *]), *) => {
            $(
                let mut board = Board::from_fen($fen).unwrap();
                for (depth, nodes) in [$($nodes), *].iter().enumerate() {
                    assert_eq!(perft(depth + 1, &mut board), *nodes);
                }
            )*
        };
    }

    #[test]
    fn test_perft() {
        assert_perft!(
            STARTING_FEN,
            [20, 400, 8902, 197281, 4865609],
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            [48, 2039, 97862, 4085603],
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 ",
            [14, 191, 2812, 43238, 674624],
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            [6, 264, 9467, 422333],
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8  ",
            [44, 1486, 62379, 2103487],
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 ",
            [46, 2079, 89890, 3894594]
        );
    }

    #[test]
    fn test_perft_960() {
        assert_perft!(
            "bqnr1kr1/pppppp1p/6p1/5n2/4B3/3N2PP/PbPPPP2/BQNR1KR1 w GDgd - 2 9",
            [31, 1132, 36559, 1261476, 43256823],
            "nqn2krb/p1prpppp/1pbp4/7P/5P2/8/PPPPPKP1/NQNRB1RB w g - 3 9",
            [21, 461, 10608, 248069, 6194124],
            "bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9",
            [21, 528, 12189, 326672, 8146062],
            "1nbbnrkr/p1p1ppp1/3p4/1p3P1p/3Pq2P/8/PPP1P1P1/QNBBNRKR w HFhf - 0 9",
            [28, 1120, 31058, 1171749, 34030312]
        );
    }
}
