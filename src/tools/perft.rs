use crate::board::{Board, movegen::MoveGenKind};

pub fn perft(depth: usize, board: &mut Board) -> usize {
    let clock = std::time::Instant::now();
    let mut nodes_count = 0;

    for m in board.generate_moves(MoveGenKind::All).iter() {
        debug_assert!(
            board.is_legal(m.mv),
            "Tried making move: {}\n{}",
            m.mv,
            board
        );

        board.make_move(m.mv);
        let divided_nodes = perft_divide(depth - 1, board);
        println!("{}: {divided_nodes}", m.mv);
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

    #[test]
    fn test_perft() {
        let mut board = Board::from_fen(STARTING_FEN).unwrap();

        assert_eq!(perft(1, &mut board), 20);
        assert_eq!(perft(2, &mut board), 400);
        assert_eq!(perft(3, &mut board), 8902);
        assert_eq!(perft(4, &mut board), 197281);
        assert_eq!(perft(5, &mut board), 4865609);

        let mut kiwipete =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ")
                .unwrap();

        assert_eq!(perft(1, &mut kiwipete), 48);
        assert_eq!(perft(2, &mut kiwipete), 2039);
        assert_eq!(perft(3, &mut kiwipete), 97862);
        assert_eq!(perft(4, &mut kiwipete), 4085603);

        let mut board3 = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 ").unwrap();

        assert_eq!(perft(1, &mut board3), 14);
        assert_eq!(perft(2, &mut board3), 191);
        assert_eq!(perft(3, &mut board3), 2812);
        assert_eq!(perft(4, &mut board3), 43238);
        assert_eq!(perft(5, &mut board3), 674624);

        let mut board4 =
            Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1")
                .unwrap();

        assert_eq!(perft(1, &mut board4), 6);
        assert_eq!(perft(2, &mut board4), 264);
        assert_eq!(perft(3, &mut board4), 9467);
        assert_eq!(perft(4, &mut board4), 422333);

        let mut board5 =
            Board::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8  ").unwrap();

        assert_eq!(perft(1, &mut board5), 44);
        assert_eq!(perft(2, &mut board5), 1486);
        assert_eq!(perft(3, &mut board5), 62379);
        assert_eq!(perft(4, &mut board5), 2103487);

        let mut board6 = Board::from_fen(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 ",
        )
        .unwrap();

        assert_eq!(perft(1, &mut board6), 46);
        assert_eq!(perft(2, &mut board6), 2079);
        assert_eq!(perft(3, &mut board6), 89890);
        assert_eq!(perft(4, &mut board6), 3894594);
    }
}
