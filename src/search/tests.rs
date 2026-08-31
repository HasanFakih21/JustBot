use super::*;
use crate::{board::Board, search::data::RootMove};

#[test]
fn test_order_moves() {
    let data = SearchData {
        board: Board::from_fen("rnbqkb1r/pp3p2/4pnpp/1p1p2N1/1Q1P4/BP2P3/P1PN1PPP/R3K2R b KQkq - 0 1").unwrap(),
        ..Default::default()
    };

    let mut move_picker = MovePicker::new(None);
    let first_move = move_picker.next(&data, false, 0).unwrap();

    assert_eq!(first_move, Move::new(Square::F8, Square::B4, MoveKind::Capture));

    let data = SearchData {
        board: Board::from_fen("rnbq1rk1/pN1p1ppp/4n2b/2p1p3/N1BP3R/2P2Q2/PP3PPP/2B1K2R w K - 0 1").unwrap(),
        ..Default::default()
    };
    let mut move_picker = MovePicker::new(None);
    let first_move = move_picker.next(&data, false, 0).unwrap();

    assert_eq!(first_move, Move::new(Square::B7, Square::D8, MoveKind::Capture));
}

#[test]
fn test_mate_in_four() {
    let mut data = SearchData::default();
    let board = Board::from_fen("6k1/5pp1/5n1p/8/5P1q/2RQ3P/B5PK/8 b - - 0 36").unwrap();
    data.board = board;

    data.time_settings().nodes = 8000;
    data.time.set_nodes_limit();
    data.root_moves = data
        .board
        .generate_moves(crate::board::movegen::MoveGenKind::All)
        .iter()
        .map(|e| RootMove {
            m: e.mv,
            ..Default::default()
        })
        .collect();

    search_runner(&mut data);
    let best_move = data.best_move.unwrap();

    println!("Best Move: {}", best_move.to_uci(&data.board));
    assert_eq!(Move::new(Square::F6, Square::G4, MoveKind::QuietMove), best_move);
}

#[test]
fn test_pv_line() {
    use MoveKind::*;
    use Square::*;

    let mut data = SearchData::default();
    let board = Board::from_fen("6k1/5pp1/5n1p/8/5P1q/2RQ3P/B5PK/8 b - - 0 36").unwrap();
    data.board = board;
    data.root_moves = data
        .board
        .generate_moves(crate::board::movegen::MoveGenKind::All)
        .iter()
        .map(|e| RootMove {
            m: e.mv,
            ..Default::default()
        })
        .collect();
    data.time_settings().nodes = 20000;
    data.time.set_nodes_limit();

    search_runner(&mut data);

    let best_move = data.best_move.unwrap();
    println!("PV: {:?}", data.pv.line());
    let mut pv_line = MoveList::new();
    pv_line.push(Move::new(F6, G4, QuietMove));
    pv_line.push(Move::new(H2, G1, QuietMove));
    pv_line.push(Move::new(H4, F2, QuietMove));
    pv_line.push(Move::new(G1, H1, QuietMove));
    pv_line.push(Move::new(F2, E1, QuietMove));
    pv_line.push(Move::new(D3, F1, QuietMove));
    pv_line.push(Move::new(E1, F1, Capture));

    let pv_display = {
        let mut output = format!("{} ", data.root_moves[0].m.to_uci(&data.board));
        for m in &data.root_moves[0].pv.inner {
            output = format!("{output}{} ", m.to_uci(&data.board));
        }

        output
    };

    let mut ver_pv = String::new();
    for m in pv_line.iter() {
        ver_pv.push_str(&format!("{} ", m.mv.to_uci(&data.board)));
    }

    assert_eq!(ver_pv, pv_display);

    assert_eq!(Move::new(Square::F6, Square::G4, MoveKind::QuietMove), best_move);
}

#[test]
fn test_bugged_position() {
    let mut board = Board::from_fen("6k1/5pp1/7p/8/5Pn1/2R4P/B5P1/4qQ1K b - - 6 39").unwrap();
    println!("Hash: {}", board.state.keys.full);
    // Position hash: 6128121706435820836

    board = Board::from_fen("6k1/5pp1/7p/8/5Pn1/2RQ3P/B4qP1/6K1 w - - 3 38").unwrap();
    println!("Hash 2: {}", board.state.keys.full);
    // Position hash: 16381162810209017462

    board = Board::from_fen("6k1/5pp1/7p/8/5Pnq/2RQ3P/B5P1/6K1 b - - 2 37").unwrap();
    println!("Hash 3: {}", board.state.keys.full);
    // Position hash: 3246015867840709621
}

#[test]
fn test_transposition_timeout() {
    let mut data = SearchData::default();
    data.time_settings().btime = Some(8080);
    let board = Board::from_fen("6k1/2p5/4R1pp/1p1r4/pP1p4/P5PP/2P2P2/6K1 b - - 0 32").unwrap();
    data.time
        .set_time_limits(board.state.side_to_move, board.state.full_move);
    data.board = board;
    data.root_moves = data
        .board
        .generate_moves(crate::board::movegen::MoveGenKind::All)
        .iter()
        .map(|e| RootMove {
            m: e.mv,
            ..Default::default()
        })
        .collect();

    search_runner(&mut data);
    data.shared.status.run();
    println!();
    data.shared.status.run();
    search_runner(&mut data);
    println!();
    data.shared.status.run();
    search_runner(&mut data);
    println!();
    data.shared.status.run();
    search_runner(&mut data);
    println!();
    data.shared.status.run();
    search_runner(&mut data);
    println!();
}
