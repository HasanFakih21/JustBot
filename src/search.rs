use crate::board::Board;
use crate::evaluation::{mated, mating};
use crate::search::data::{SearchData, Status};
use crate::search::movepicker::MovePicker;
use crate::types::plytable::PlyTable;
use crate::types::stackvec::StackVec;
use crate::types::*;

pub mod data;
pub mod movepicker;
pub mod time;

#[cfg(test)]
mod tests;

impl Board {
    //Needs fixing
    pub fn detect_repetitions(&self) -> usize {
        let half_moves = self.state.half_move_clock as usize;
        let mut count = 0;

        if self.game_history.len() < half_moves {
            return 0;
        }

        let last_halfmove_ply = self.game_history.len() - half_moves;
        for position in self.game_history[last_halfmove_ply..].iter() {
            if self.state.hash == *position {
                count += 1
            }
        }

        count
    }
}

pub trait NodeType {
    const PV: bool;
    const ROOT: bool;
}

pub struct PV;
pub struct Root;
pub struct NonPV;

impl NodeType for PV {
    const PV: bool = true;
    const ROOT: bool = false;
}

impl NodeType for NonPV {
    const PV: bool = false;
    const ROOT: bool = false;
}

impl NodeType for Root {
    const PV: bool = true;
    const ROOT: bool = true;
}

pub fn search_runner(data: &mut SearchData) -> Option<MoveEntry> {
    data.reset_pv();
    data.start_time();
    data.clear_features();
    data.initialize_nnue();
    data.nodes = 0;

    let mut depth = 1;

    //Initialize with move from first depth
    let best_score = search::<Root>(data, depth, -INFINITY, INFINITY, 0);
    let mut best_move = data.get_pv().get(0);
    depth += 1;

    //Aspiration Window
    let mut score = best_score;
    let mut alpha_window = 25;
    let mut beta_window = 25;
    let mut alpha = score - alpha_window;
    let mut beta = score + beta_window;

    //All infos belonging to the pv should be sent together e.g. info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3
    if data.report {
        //Report mate score
        let score_print = if score.abs() > MATE_CUTOFF {
            let num_plies = MATE_SCORE - score.abs();
            let mate_in = score.signum() * ((num_plies + 1) / 2);
            format!("mate {}", mate_in)
        } else {
            format!("cp {}", score)
        };

        println!(
            "info depth {} time {} score {} nodes {} nps {} pv {} hashfull {}",
            depth - 1,
            data.time.elapsed().as_millis(),
            score_print,
            data.shared.get_total_nodes_searched(),
            data.nodes_per_second(),
            data.get_pv(),
            data.shared.tt.hashfull(),
        );
    }

    //Iterative Deepening
    loop {
        data.ply_table = PlyTable::new();

        if data.time.hard_limit(data.nodes)
            || data
                .time
                .node_limit()
                .is_some_and(|node_limit| data.shared.get_total_nodes_searched() >= node_limit)
            || depth > data.time.depth_limit()
            || data.shared.status.get() == Status::STOPPED
        {
            break;
        }

        let deeper_move_score = search::<Root>(data, depth, alpha, beta, 0);

        let new_score = deeper_move_score;
        if new_score <= alpha {
            //Failed Low
            alpha_window *= 2;
            alpha -= alpha_window;
            continue;
        } else if new_score >= beta {
            //Failed High
            beta_window *= 2;
            beta += beta_window;
            continue;
        }

        depth += 1;

        score = new_score;
        best_move = data.get_pv().get(0);
        best_move.score = score;
        alpha_window = 25;
        beta_window = 25;
        alpha = score - alpha_window;
        beta = score + beta_window;
        if data.report {
            //Report mate score
            let score_print = if score.abs() > MATE_CUTOFF {
                let num_plies = MATE_SCORE - score.abs();
                let mate_in = score.signum() * ((num_plies + 1) / 2);
                format!("mate {}", mate_in)
            } else {
                format!("cp {}", score)
            };

            println!(
                "info depth {} time {} score {} nodes {} nps {} pv {} hashfull {}",
                depth - 1,
                data.time.elapsed().as_millis(),
                score_print,
                data.shared.get_total_nodes_searched(),
                data.nodes_per_second(),
                data.get_pv(),
                data.shared.tt.hashfull()
            );
        }

        if data.time.soft_limit() {
            break;
        }
    }

    Some(best_move)
}

pub fn search<Node: NodeType>(
    data: &mut SearchData,
    depth: i32,
    mut alpha: i32,
    beta: i32,
    ply: isize,
) -> i32 {
    let stm = data.board.state.side_to_move;

    if depth <= 0 {
        if data.board.king_in_check() {
            return search_checks(data, alpha, beta, ply);
        } else {
            return quiesce(data, alpha, beta, ply); //Horizon Node
        }
    }

    data.increment_nodes();

    if Node::PV && !Node::ROOT {
        data.clear_pv(ply);
    }

    if data.board.state.half_move_clock > 4 && !Node::ROOT {
        //50 move rule
        if data.board.state.half_move_clock >= 100 {
            return 0;
        }
        //We need to check history if positions were repeated only for the side to move.
        let count = data.board.detect_repetitions();
        if count >= 2 {
            return 0;
        }
    }

    let tt_entry = data.shared.tt.get_entry(data.board.state.hash);

    //TT Cutoffs only if depth of entry is greater or equal to the depth of the current node
    if let Some(e) = tt_entry
        && !Node::PV
        && e.get_depth() >= depth
        && e.get_score().abs() < MATE_CUTOFF
    //Mate scores need to be properly adjusted for cutoffs
    {
        let tt_score = e.get_score();
        match e.get_bound() {
            Bound::Exact => return tt_score,
            Bound::Lower => {
                if tt_score >= beta {
                    return tt_score;
                }
            }
            Bound::Upper => {
                if tt_score < alpha {
                    return tt_score;
                }
            }
            _ => unreachable!(),
        }
    }

    let in_check = data.board.king_in_check();
    let static_eval = if in_check {
        -INFINITY
    } else if let Some(e) = tt_entry {
        e.get_eval()
    } else {
        data.nnue_evaluate()
    };

    data.ply_table[ply].eval = static_eval;
    let improving = if in_check {
        false
    } else if data.ply_table[ply - 2].eval != -INFINITY {
        (static_eval - data.ply_table[ply - 2].eval) > 0
    } else if data.ply_table[ply - 4].eval != -INFINITY {
        (static_eval - data.ply_table[ply - 4].eval) > 0
    } else {
        false
    };

    //Reverse Futillity Pruning (RFP)
    if !in_check && !Node::PV && depth < 7 {
        let margin = 150 * depth - (100 * improving as i32);
        if static_eval >= beta + margin {
            return static_eval;
        }
    }

    //Null Move Pruning
    if !Node::PV
        && !in_check
        && !data.board.only_king_and_pawns()
        && static_eval >= beta - 50 * improving as i32
        && !data.ply_table[ply - 1].m.is_null()
    {
        let r = 4;
        data.ply_table[ply].conthistory = data.ply_table.sentinel();
        data.ply_table[ply].m = Move::default();
        data.ply_table[ply].piece = None;

        data.board.make_null_move();
        let null_move_score =
            -search::<NonPV>(data, depth.saturating_sub(r), -beta, -(beta - 1), ply + 1);
        data.board.unmake_move();
        if null_move_score >= beta {
            return null_move_score;
        }
    }

    let mut move_count = 0;
    let mut best_score = -INFINITY;
    let mut best_move: Option<Move> = None;
    let mut bound = Bound::Upper; //Fail-high means score is atleast this good so lower-bound/Fail-low means the score is an upper bound
    let tt_move = data
        .shared
        .tt
        .get_entry(data.board.state.hash)
        .map(|e| e.get_best_move());

    let mut move_picker = MovePicker::new(tt_move);
    let mut quiets_searched = StackVec::<Move, 256>::new();
    let mut noisies_searched = StackVec::<Move, 256>::new();
    let mut skip_quiets = false;

    while let Some(m) = move_picker.next(data, skip_quiets, ply) {
        move_count += 1;
        let is_quiet = m.get_kind().is_quiet();

        if !Node::ROOT && !mated(best_score) {
            //Late Move Pruning (LMP)
            if !in_check
                && !mating(beta)
                && is_quiet
                && move_count > (3 + depth as usize * depth as usize) / (2 - (improving as usize))
            {
                skip_quiets = true;
                continue;
            }

            //Futility Pruning (FP)
            if !in_check && is_quiet && depth < 6 && static_eval + 100 * depth + 150 <= alpha {
                skip_quiets = true;
                continue;
            }
        }

        //Make Move
        data.make_move(m, ply);
        let mut score = -INFINITY;

        //Late Move Reductions (LMR)
        if depth > 3 && move_count > 1 && !Node::PV {
            let mut r = LMR_TABLE[is_quiet as usize][depth as usize][move_count];
            r += 200 * !improving as i32;

            let reduction = r / 1024;
            let reduced_depth = (depth - 1) - reduction;

            score = -search::<NonPV>(data, reduced_depth, -alpha - 1, -alpha, ply + 1);
            if score > alpha && reduced_depth < depth - 1 {
                score = -search::<NonPV>(data, depth - 1, -alpha - 1, -alpha, ply + 1);
            }
        } else if !Node::PV || move_count > 1 {
            score = -search::<NonPV>(data, depth - 1, -alpha - 1, -alpha, ply + 1);
        }

        //Principal Variation Search (PVS)
        if Node::PV && (move_count == 1 || score > alpha) {
            score = -search::<PV>(data, depth - 1, -beta, -alpha, ply + 1);
        }

        //Unmake Move
        data.unmake_move(m);

        if data.time.hard_limit(data.nodes)
            || data.shared.status.get() == Status::STOPPED
            || data
                .time
                .node_limit()
                .is_some_and(|node_limit| data.shared.get_total_nodes_searched() >= node_limit)
        {
            return TIMEOUT_SCORE;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = Some(m);
                bound = Bound::Exact;
                data.add_pv_move(m, ply);

                //Cutoff
                if score >= beta {
                    bound = Bound::Lower;
                    break;
                }

                alpha = score;
            }
        }

        //Add searched quiet/noisy moves to list
        if let Some(bm) = best_move
            && m != bm
        {
            if is_quiet {
                quiets_searched.push(m);
            } else {
                noisies_searched.push(m);
            }
        }
    }

    if move_count == 0 {
        if in_check {
            return -MATE_SCORE + ply as i32;
        } else {
            return 0;
        }
    }

    if let Some(m) = best_move {
        let is_quiet = m.get_kind().is_quiet();

        let quiet_bonus = 300 * depth - 250;
        let quiet_malus = 300 * depth - 250;

        let noisy_bonus = (250 * depth).min(1000) - 250;
        let noisy_malus = (300 * depth).min(1000) - 250;

        let cont_bonus = (350 * depth).min(1000) - 250;
        let cont_malus = (250 * depth).min(1000) - 250;

        let threats = data.board.state.threats;

        if is_quiet {
            //Add quiet bonus to history
            data.quiet_history.update(threats, stm, m, quiet_bonus);
            //Conthistory Bonus
            data.update_conthistories(m, ply, cont_bonus);
            //Add malus to quiet moves
            for e in quiets_searched.iter() {
                let quiet_move = e;
                data.quiet_history
                    .update(threats, stm, *quiet_move, -quiet_malus);

                //Conthistory malus
                data.update_conthistories(*quiet_move, ply, -cont_malus);
            }
        } else {
            //Add noisy bonus to history
            let piece = data.board.get_piece_at_square(m.get_from());
            let to = m.get_to();
            let captured = data
                .board
                .get_piece_at_square(m.get_capture_square())
                .map(|e| e.1);
            data.noisy_history
                .update(piece, to, captured, threats, noisy_bonus);
        }

        //Add malus to noisy moves
        for m in noisies_searched.iter() {
            let piece = data.board.get_piece_at_square(m.get_from());
            let to = m.get_to();
            let captured = data
                .board
                .get_piece_at_square(m.get_capture_square())
                .map(|e| e.1);
            data.noisy_history
                .update(piece, to, captured, threats, -noisy_malus);
        }

        let tt_score = best_score;
        data.shared.tt.add_entry(
            m,
            tt_score,
            static_eval,
            bound,
            data.board.state.hash,
            depth,
            ply,
            Node::PV,
        );
    }

    best_score
}

pub fn quiesce(data: &mut SearchData, mut alpha: i32, beta: i32, ply: isize) -> i32 {
    data.increment_nodes();
    let mut best_score = data.nnue_evaluate();

    if ply >= MAX_PLY as isize - 1 {
        return best_score;
    }

    if best_score >= beta {
        return best_score;
    }

    if best_score > alpha {
        alpha = best_score;
    }

    let tt_move = data
        .shared
        .tt
        .get_entry(data.board.state.hash)
        .map(|e| e.get_best_move());
    let mut move_picker = MovePicker::new(tt_move);

    while let Some(m) = move_picker.next(data, true, ply) {
        //Static Exchange Evaluation Pruning (SEE Pruning)
        if !mated(best_score) && !data.board.see(m, -150) {
            continue;
        }

        data.make_move(m, ply);
        let score = -quiesce(data, -beta, -alpha, ply + 1);
        data.unmake_move(m);

        if data.time.hard_limit(data.nodes)
            || data.shared.status.get() == Status::STOPPED
            || data
                .time
                .node_limit()
                .is_some_and(|node_limit| data.shared.get_total_nodes_searched() >= node_limit)
        {
            return TIMEOUT_SCORE;
        }

        if score >= beta {
            //Add noisy bonus to history
            let piece = data.board.get_piece_at_square(m.get_from());
            let to = m.get_to();
            let captured = data
                .board
                .get_piece_at_square(m.get_capture_square())
                .map(|e| e.1);
            data.noisy_history
                .update(piece, to, captured, data.board.state.threats, 100);

            return score;
        }

        if score > best_score {
            best_score = score;
        }

        if score > alpha {
            alpha = score;
        }
    }

    best_score
}

pub fn search_checks(data: &mut SearchData, mut alpha: i32, beta: i32, ply: isize) -> i32 {
    let mut best_score = -INFINITY;
    let mut move_count = 0;

    data.increment_nodes();

    if data.board.state.half_move_clock > 4 {
        //50 move rule
        if data.board.state.half_move_clock >= 100 {
            return 0;
        }
        //We need to check history if positions were repeated only for the side to move.
        let count = data.board.detect_repetitions();
        if count >= 2 {
            return 0;
        }
    }

    let in_check = data.board.king_in_check();

    if ply >= MAX_PLY as isize - 1 {
        return if in_check { 0 } else { data.nnue_evaluate() };
    }

    if !in_check {
        return quiesce(data, alpha, beta, ply);
    }

    let tt_move = data
        .shared
        .tt
        .get_entry(data.board.state.hash)
        .map(|e| e.get_best_move());
    let mut move_picker = MovePicker::new(tt_move);

    while let Some(m) = move_picker.next(data, false, ply) {
        move_count += 1;

        data.make_move(m, ply);
        let score = -search_checks(data, -beta, -alpha, ply + 1);

        data.unmake_move(m);

        if data.time.hard_limit(data.nodes)
            || data.shared.status.get() == Status::STOPPED
            || data
                .time
                .node_limit()
                .is_some_and(|node_limit| data.shared.get_total_nodes_searched() >= node_limit)
        {
            return TIMEOUT_SCORE;
        }

        if score >= beta {
            //Add noisy bonus to history
            let piece = data.board.get_piece_at_square(m.get_from());
            let to = m.get_to();
            let captured = data
                .board
                .get_piece_at_square(m.get_capture_square())
                .map(|e| e.1);
            data.noisy_history
                .update(piece, to, captured, data.board.state.threats, 100);

            return score;
        }

        if score > best_score {
            best_score = score;
        }

        if score > alpha {
            alpha = score;
        }
    }

    if move_count == 0 {
        if data.board.king_in_check() {
            return -MATE_SCORE + ply as i32;
        } else {
            return 0;
        }
    }

    best_score
}
