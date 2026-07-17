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

pub fn search_runner(data: &mut SearchData) {
    data.reset_pv();
    data.start_time();
    data.clear_features();
    data.initialize_nnue();

    let mut alpha_window = 25;
    let mut beta_window = 25;
    let mut alpha = -Score::INFINITY;
    let mut beta = Score::INFINITY;

    let mut depth = 1;
    let mut best_move = None;

    //Iterative Deepening
    loop {
        data.ply_table = PlyTable::new();

        if data.time.hard_limit(data.nodes(), data.id)
            || data
                .time
                .node_limit()
                .is_some_and(|node_limit| data.nodes() >= node_limit)
            || depth > data.time.depth_limit()
            || data.shared.status.get() == Status::STOPPED
        {
            if data.id == 0 {
                data.shared.status.stop();
            }

            break;
        }

        let score = search::<Root>(data, depth, alpha, beta, 0);

        //Aspiration Window
        if score <= alpha {
            //Failed Low
            alpha_window *= 2;
            alpha -= alpha_window;
            continue;
        } else if score >= beta {
            //Failed High
            beta_window *= 2;
            beta += beta_window;
            continue;
        }

        depth += 1;
        best_move = data.pv.line().first().copied();
        data.print_uci_info(score, depth);

        if data.time.soft_limit() {
            if data.id == 0 {
                data.shared.status.stop();
            }

            break;
        }

        alpha_window = 25;
        beta_window = 25;
        alpha = score - alpha_window;
        beta = score + beta_window;
    }

    data.best_move = best_move;
}

pub fn search<Node: NodeType>(
    data: &mut SearchData,
    depth: i32,
    mut alpha: i32,
    beta: i32,
    ply: isize,
) -> i32 {
    if Node::PV && !Node::ROOT {
        data.pv.clear(ply);
    }

    if data.shared.status.get() == Status::STOPPED {
        return Score::TIMEOUT;
    }

    if depth <= 0 {
        return quiesce::<Node>(data, alpha, beta, ply); //Horizon Node
    }

    data.shared.increment_nodes(data.id);

    let stm = data.board.state.side_to_move;
    let in_check = data.board.king_in_check();

    if !Node::ROOT {
        //Check for draws
        if is_draw(data) {
            return 0;
        }

        if ply >= MAX_PLY as isize - 1 {
            if in_check {
                return 0;
            } else {
                return data.nnue_evaluate();
            }
        }
    }

    //Check for Time Outs
    if data.time.hard_limit(data.nodes(), data.id)
        || data.shared.status.get() == Status::STOPPED
        || data
            .time
            .node_limit()
            .is_some_and(|node_limit| data.nodes() >= node_limit)
    {
        data.shared.status.stop();
        return Score::TIMEOUT;
    }

    let tt_entry = data.shared.tt.get_entry(data.board.state.hash, ply);

    //TT Cutoffs only if depth of entry is greater or equal to the depth of the current node
    if let Some(e) = &tt_entry
        && !Node::PV
        && e.get_depth() >= depth
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

    let static_eval = if in_check {
        -Score::INFINITY
    } else if let Some(e) = &tt_entry
        && e.get_eval() != -Score::INFINITY
    {
        e.get_eval()
    } else {
        data.nnue_evaluate()
    };

    data.ply_table[ply].eval = static_eval;
    let improving = if in_check {
        false
    } else if data.ply_table[ply - 2].eval != -Score::INFINITY {
        (static_eval - data.ply_table[ply - 2].eval) > 0
    } else if data.ply_table[ply - 4].eval != -Score::INFINITY {
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
    let mut best_score = -Score::INFINITY;
    let mut best_move: Option<Move> = None;
    let mut bound = Bound::Upper; //Fail-high means score is atleast this good so lower-bound/Fail-low means the score is an upper bound
    let tt_move = tt_entry.map(|e| e.get_best_move()).filter(|m| !m.is_null());

    let mut move_picker = MovePicker::new(tt_move);
    let mut quiets_searched = StackVec::<Move, 32>::new();
    let mut noisies_searched = StackVec::<Move, 32>::new();
    let mut skip_quiets = false;

    while let Some(m) = move_picker.next(data, skip_quiets, ply) {
        move_count += 1;
        let is_direct_check = data.board.is_direct_check(m);
        let is_quiet = m.get_kind().is_quiet();

        if !Node::ROOT && !mated(best_score) {
            //Late Move Pruning (LMP)
            if !in_check
                && !is_direct_check
                && !mating(beta)
                && is_quiet
                && move_count > (3 + depth as usize * depth as usize) / (2 - (improving as usize))
            {
                skip_quiets = true;
                continue;
            }

            //Futility Pruning (FP)
            if !in_check
                && !is_direct_check
                && is_quiet
                && depth < 6
                && static_eval + 100 * depth + 150 <= alpha
            {
                skip_quiets = true;
                continue;
            }
        }

        //Make Move
        data.make_move(m, ply);
        let new_depth = (depth - 1) + (in_check as i32);
        let mut score = -Score::INFINITY;

        //Late Move Reductions (LMR)
        if depth > 3 && move_count > 1 {
            let mut r = LMR_TABLE[is_quiet as usize][depth.min(127) as usize][move_count.min(63)];
            r += 200 * !improving as i32;

            let reduction = r / 1024;
            let reduced_depth = (new_depth - reduction).max(1) + 2 * Node::PV as i32;

            score = -search::<NonPV>(data, reduced_depth, -alpha - 1, -alpha, ply + 1);
            if score > alpha && reduced_depth < new_depth {
                score = -search::<NonPV>(data, new_depth, -alpha - 1, -alpha, ply + 1);
            }
        } else if !Node::PV || move_count > 1 {
            score = -search::<NonPV>(data, new_depth, -alpha - 1, -alpha, ply + 1);
        }

        //Principal Variation Search (PVS)
        if Node::PV && (move_count == 1 || score > alpha) {
            score = -search::<PV>(data, new_depth, -beta, -alpha, ply + 1);
        }

        //Unmake Move
        data.unmake_move(m);

        if data.shared.status.get() == Status::STOPPED {
            return Score::TIMEOUT;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = Some(m);
                bound = Bound::Exact;
                data.pv.add(m, ply);

                //Cutoff
                if score >= beta {
                    bound = Bound::Lower;
                    break;
                }

                alpha = score;
            }
        }

        //Add searched quiet/noisy moves to list
        if best_move != Some(m) && move_count < 32 {
            if is_quiet {
                quiets_searched.push(m);
            } else {
                noisies_searched.push(m);
            }
        }
    }

    if move_count == 0 {
        if in_check {
            return -Score::MATE + ply as i32;
        } else {
            return 0;
        }
    }

    if let Some(m) = best_move {
        let is_quiet = m.get_kind().is_quiet();

        let quiet_bonus = (300 * depth).min(1000) - 250;
        let quiet_malus = (300 * depth).min(1000) - 250;

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
    }

    data.shared.tt.add_entry(
        best_move.unwrap_or_default(),
        best_score,
        static_eval,
        bound,
        data.board.state.hash,
        depth,
        ply,
        Node::PV,
    );

    best_score
}

pub fn quiesce<Node: NodeType>(
    data: &mut SearchData,
    mut alpha: i32,
    beta: i32,
    ply: isize,
) -> i32 {
    data.shared.increment_nodes(data.id);
    if is_draw(data) {
        return 0;
    }

    if data.time.hard_limit(data.nodes(), data.id)
        || data.shared.status.get() == Status::STOPPED
        || data
            .time
            .node_limit()
            .is_some_and(|node_limit| data.nodes() >= node_limit)
    {
        data.shared.status.stop();
        return Score::TIMEOUT;
    }

    let tt_entry = data.shared.tt.get_entry(data.board.state.hash, ply);

    //TT Cutoffs
    if let Some(e) = &tt_entry
        && !Node::PV
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

    if ply >= MAX_PLY as isize - 1 {
        if in_check {
            return 0;
        } else {
            return data.nnue_evaluate();
        }
    }

    let mut best_score = if in_check {
        -Score::INFINITY
    } else if let Some(e) = &tt_entry
        && e.get_eval() != -Score::INFINITY
    {
        e.get_eval()
    } else {
        data.nnue_evaluate()
    };

    let static_eval = best_score;

    //Stand Pat
    if best_score >= beta {
        return best_score;
    }

    if best_score > alpha {
        alpha = best_score;
    }

    let tt_move = tt_entry.map(|e| e.get_best_move()).filter(|m| !m.is_null());

    let mut move_picker = MovePicker::new(tt_move);
    let mut move_count = 0;
    let mut bound = Bound::Upper;
    let mut best_move: Option<Move> = None;
    let skip_quiets = !in_check;

    while let Some(m) = move_picker.next(data, skip_quiets, ply) {
        move_count += 1;

        if !mated(best_score) {
            //Late Move Pruning (LMP)
            if move_count >= 4 {
                break;
            }

            //Static Exchange Evaluation Pruning (SEE Pruning)
            if !data.board.see(m, -150) {
                continue;
            }
        }

        data.make_move(m, ply);
        let score = -quiesce::<Node>(data, -beta, -alpha, ply + 1);
        data.unmake_move(m);

        if data.shared.status.get() == Status::STOPPED {
            return Score::TIMEOUT;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = Some(m);

                //Cutoff
                if score >= beta {
                    bound = Bound::Lower;
                    break;
                }

                alpha = score;
            }
        }
    }

    if in_check && move_count == 0 {
        return -Score::MATE + ply as i32;
    }

    if best_score >= beta
        && let Some(m) = best_move
        && !m.get_kind().is_quiet()
    {
        //Add noisy bonus to history
        let piece = data.board.get_piece_at_square(m.get_from());
        let to = m.get_to();
        let captured = data
            .board
            .get_piece_at_square(m.get_capture_square())
            .map(|e| e.1);
        data.noisy_history
            .update(piece, to, captured, data.board.state.threats, 100);
    }

    data.shared.tt.add_entry(
        best_move.unwrap_or_default(),
        best_score,
        static_eval,
        bound,
        data.board.state.hash,
        0,
        ply,
        Node::PV,
    );

    best_score
}
