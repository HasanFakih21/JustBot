use super::*;
use crate::board::Board;

#[test]
fn test_search() {
    let mut data = SearchData::default();
    search::<Root>(&mut data, 5, -Score::INFINITY, Score::INFINITY, 0);
    let best_move = data.get_pv().get(0).mv;
    println!("Best move: {}", best_move);
}

#[test]
fn test_order_moves() {
    let data = SearchData {
        board: Board::from_fen(
            "rnbqkb1r/pp3p2/4pnpp/1p1p2N1/1Q1P4/BP2P3/P1PN1PPP/R3K2R b KQkq - 0 1",
        )
        .unwrap(),
        ..Default::default()
    };

    let mut move_picker = MovePicker::new(None);
    let first_move = move_picker.next(&data, false, 0).unwrap();

    assert_eq!(
        first_move,
        Move::new(Square::F8, Square::B4, MoveKind::Capture)
    );

    let data = SearchData {
        board: Board::from_fen("rnbq1rk1/pN1p1ppp/4n2b/2p1p3/N1BP3R/2P2Q2/PP3PPP/2B1K2R w K - 0 1")
            .unwrap(),
        ..Default::default()
    };
    let mut move_picker = MovePicker::new(None);
    let first_move = move_picker.next(&data, false, 0).unwrap();

    assert_eq!(
        first_move,
        Move::new(Square::B7, Square::D8, MoveKind::Capture)
    );
}

#[test]
fn test_repetion_detection() {
    use MoveKind::*;
    use Square::*;

    let mut board = Board::from_fen("8/6K1/3N4/8/5Q2/8/1kr5/8 w - - 0 1").unwrap();
    board.make_move(Move::new(F4, E4, QuietMove));
    board.make_move(Move::new(C2, C1, QuietMove));
    board.make_move(Move::new(E4, F4, QuietMove));
    board.make_move(Move::new(C1, C2, QuietMove));
    board.make_move(Move::new(F4, E4, QuietMove));
    board.make_move(Move::new(C2, C1, QuietMove));
    board.make_move(Move::new(E4, F4, QuietMove));

    println!("{}", board);

    let mut data = SearchData {
        board,
        ..Default::default()
    };

    data.clear_features();
    data.initialize_nnue();

    let score = search::<Root>(&mut data, 3, -Score::INFINITY, Score::INFINITY, 0);
    println!("{score}");
    let m = data.get_best_move();

    println!(
        "{:?}\nCurrent Hash: {}",
        data.board.game_history, data.board.state.hash
    );
    println!("Repetions counted: {}", data.board.detect_repetitions());
    assert_eq!(score, 0);
    assert_eq!(m, Move::new(C1, C2, QuietMove));
}

#[test]
fn test_mate_in_one() {
    let mut data = SearchData::default();
    let board =
        Board::from_fen("r1b4r/p1p1q3/1bppk3/4pp2/3PP1Q1/2P1R3/PP3PPP/RN4K1 w - - 0 18").unwrap();
    data.board = board;

    search::<Root>(&mut data, 1, -Score::INFINITY, Score::INFINITY, 0);
    let best_move = data.get_pv().get(0).mv;
    println!("Best Move: {}", best_move);
    assert_eq!(
        Move::new(Square::G4, Square::F5, MoveKind::Capture),
        best_move
    );
}

#[test]
fn test_mate_in_four() {
    let mut data = SearchData::default();
    let board = Board::from_fen("6k1/5pp1/5n1p/8/5P1q/2RQ3P/B5PK/8 b - - 0 36").unwrap();
    data.board = board;

    data.get_time_settings().nodes = 8000;
    data.time.set_nodes_limit();

    let best_move = search_runner(&mut data).unwrap().mv;

    println!("Best Move: {}", best_move);
    assert_eq!(
        Move::new(Square::F6, Square::G4, MoveKind::QuietMove),
        best_move
    );
}

#[test]
fn test_pv_line() {
    use MoveKind::*;
    use Square::*;

    let mut data = SearchData::default();
    let board = Board::from_fen("6k1/5pp1/5n1p/8/5P1q/2RQ3P/B5PK/8 b - - 0 36").unwrap();
    data.board = board;

    data.get_time_settings().nodes = 20000;
    data.time.set_nodes_limit();

    let score = search_runner(&mut data).unwrap().score;

    let best_move = data.get_best_move();
    println!("PV: {}", data.get_pv());
    println!("Eval: {}", score);
    let mut pv_line = MoveList::new();
    pv_line.push(Move::new(F6, G4, QuietMove));
    pv_line.push(Move::new(H2, G1, QuietMove));
    pv_line.push(Move::new(H4, F2, QuietMove));
    pv_line.push(Move::new(G1, H1, QuietMove));
    pv_line.push(Move::new(F2, E1, QuietMove));
    pv_line.push(Move::new(D3, F1, QuietMove));
    pv_line.push(Move::new(E1, F1, Capture));

    assert_eq!(pv_line.to_string(), data.get_pv().to_string());

    println!("Best Move: {}", best_move);
    assert_eq!(
        Move::new(Square::F6, Square::G4, MoveKind::QuietMove),
        best_move
    );
}

#[test]
fn test_bugged_position() {
    let mut board = Board::from_fen("6k1/5pp1/7p/8/5Pn1/2R4P/B5P1/4qQ1K b - - 6 39").unwrap();
    println!("Hash: {}", board.state.hash);
    //Position hash: 6128121706435820836

    board = Board::from_fen("6k1/5pp1/7p/8/5Pn1/2RQ3P/B4qP1/6K1 w - - 3 38").unwrap();
    println!("Hash 2: {}", board.state.hash);
    //Position hash: 16381162810209017462

    board = Board::from_fen("6k1/5pp1/7p/8/5Pnq/2RQ3P/B5P1/6K1 b - - 2 37").unwrap();
    println!("Hash 3: {}", board.state.hash);
    //Position hash: 3246015867840709621
}

#[test]
fn test_transposition_timeout() {
    let mut data = SearchData::default();
    data.get_time_settings().btime = Some(8080);
    let board = Board::from_fen("6k1/2p5/4R1pp/1p1r4/pP1p4/P5PP/2P2P2/6K1 b - - 0 32").unwrap();
    data.time.set_time_limits(board.state.side_to_move);
    data.board = board;

    let _ = search_runner(&mut data);
    println!();
    let _ = search_runner(&mut data);
    println!();
    let _ = search_runner(&mut data);
    println!();
    let _ = search_runner(&mut data);
    println!();
    let _ = search_runner(&mut data);
    println!();
}
