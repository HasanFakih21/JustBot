use crate::search::data::{SearchData, Status};
use crate::search::movepicker::{MovePicker, Stage};
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
    data.network.full_refresh(&data.board);

    let mut alpha_window = 31;
    let mut beta_window = 20;
    let mut alpha = -Score::INFINITY;
    let mut beta = Score::INFINITY;

    let mut depth = 1;
    let mut best_move = None;

    if data.root_moves.is_empty() {
        data.best_move = None;
        return;
    }

    // Iterative Deepening
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

        let score = search::<Root>(data, depth, alpha, beta, 0, false);

        // Aspiration Window
        if score <= alpha {
            // Failed Low
            alpha_window *= 2;
            alpha -= alpha_window;
            continue;
        } else if score >= beta {
            // Failed High
            beta_window *= 2;
            beta += beta_window;
            continue;
        }

        depth += 1;
        best_move = data.pv.line().first().copied();
        data.print_uci_info(score, depth);

        let multiplier = || {
            (3.0 - (data
                .root_moves
                .iter()
                .find(|rm| rm.m == best_move.unwrap())
                .unwrap()
                .nodes as f32
                / data.nodes() as f32)
                * 2.5)
                .max(0.55)
        };

        if data.time.soft_limit(multiplier) {
            if data.id == 0 {
                data.shared.status.stop();
            }

            break;
        }

        alpha_window = 31;
        beta_window = 20;
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
    cutnode: bool,
) -> i32 {
    if Node::PV && !Node::ROOT {
        data.pv.clear(ply);
    }

    if data.shared.status.get() == Status::STOPPED {
        return Score::TIMEOUT;
    }

    // Horizon Node
    if depth <= 0 {
        return quiesce::<Node>(data, alpha, beta, ply);
    }

    data.shared.increment_nodes(data.id);

    let stm = data.board.state.side_to_move;
    let in_check = data.board.king_in_check();
    let excluded = !data.ply_table[ply].excluded.is_null();

    if !Node::ROOT {
        // Check for draws
        if data.board.is_draw() {
            return Score::DRAW;
        }

        if ply >= MAX_PLY as isize - 1 {
            if in_check {
                return Score::DRAW;
            } else {
                return data.network.evaluate(&data.board);
            }
        }
    }

    // Check for Time Outs
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

    let tt_entry = data.shared.tt.get_entry(data.board.state.keys.full, ply);
    let tt_move = tt_entry
        .as_ref()
        .map(|e| e.get_best_move())
        .filter(|m| !m.is_null());
    let tt_bound = tt_entry.as_ref().map(|e| e.get_bound());
    let tt_score = tt_entry.as_ref().map(|e| e.get_score());
    let tt_pv = tt_entry.as_ref().map(|e| e.is_pv()).unwrap_or(false);
    let tt_depth = tt_entry.as_ref().map(|e| e.get_depth());

    // TT Cutoffs
    if let Some(tt_score) = tt_score
        && let Some(tt_bound) = tt_bound
        && tt_score != -Score::INFINITY
        && !Node::PV
        && tt_depth.is_some_and(|d| d >= depth)
        && (tt_score <= alpha || cutnode)
        && !excluded
        && match tt_bound {
            Bound::Exact => true,
            Bound::Lower => tt_score >= beta,
            Bound::Upper => tt_score < alpha,
            _ => unreachable!(),
        }
    {
        return tt_score;
    }

    // Evaluation
    let raw_eval;
    let static_eval;
    let correction = data.correction();

    if in_check {
        raw_eval = -Score::INFINITY;
        static_eval = -Score::INFINITY;
    } else if let Some(e) = &tt_entry
        && e.get_eval() != -Score::INFINITY
    {
        raw_eval = e.get_eval();
        static_eval = raw_eval + correction;
    } else {
        raw_eval = data.network.evaluate(&data.board);
        static_eval = raw_eval + correction;
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

    // Razoring
    if !Node::PV
        && !in_check
        && tt_bound.is_none_or(|b| b != Bound::Lower)
        && static_eval < alpha - 250 - 250 * depth * depth
        && alpha < 2000
    {
        return quiesce::<Node>(data, alpha, beta, ply);
    }

    // Reverse Futillity Pruning (RFP)
    if !in_check
        && !Node::PV
        && !excluded
        && static_eval >= beta + 148 * depth - (92 * improving as i32)
    {
        return ilerp::<1024>(static_eval, beta, 700);
    }

    // Null Move Pruning
    if cutnode
        && depth >= 3
        && !excluded
        && !in_check
        && !data.board.only_king_and_pawns()
        && tt_bound.is_none_or(|b| b != Bound::Upper)
        && static_eval >= beta - 60 * improving as i32
        && !data.ply_table[ply - 1].m.is_null()
    {
        let r = 6 + depth * 128 / 640;
        data.ply_table[ply].conthistory = data.ply_table.sentinel();
        data.ply_table[ply].m = Move::default();
        data.ply_table[ply].piece = None;

        data.board.make_null_move();
        let null_move_score = -search::<NonPV>(data, depth - r, -beta, -(beta - 1), ply + 1, false);
        data.board.unmake_move();
        if null_move_score >= beta {
            return null_move_score;
        }

        if data.shared.status.get() == Status::STOPPED {
            return Score::TIMEOUT;
        }
    }

    // Singular Extensions (SE)
    let mut extension = 0;
    if !Node::ROOT
        && !excluded
        && depth >= 6
        && tt_depth.is_some_and(|d| d >= depth - 3)
        && let Some(tt_move) = tt_move
        && let Some(tt_bound) = tt_bound
        && let Some(tt_score) = tt_score
        && tt_score != -Score::INFINITY
        && !is_mate(tt_score)
        && tt_bound != Bound::Upper
    {
        let singular_depth = (depth - 1) / 2;
        let singular_beta = tt_score - (depth + depth);

        data.ply_table[ply].excluded = tt_move;
        data.ply_table[ply].m = Move::default();
        // Search everything except the TT move with a null window at a reduced depth to find out if it's worth extending or not
        let singular_score = search::<NonPV>(
            data,
            singular_depth,
            singular_beta - 1,
            singular_beta,
            ply,
            cutnode,
        );
        data.ply_table[ply].excluded = Move::default();

        if data.shared.status.get() == Status::STOPPED {
            return Score::TIMEOUT;
        }

        if singular_score < singular_beta {
            let double_margin = 10 + 250 * Node::PV as i32;
            extension = 1 + (singular_score < singular_beta - double_margin) as i32;
        }
        // Negative Extensions
        else if tt_score >= beta || cutnode {
            extension -= 2;
        }
    }

    let mut move_count = 0;
    let mut best_score = -Score::INFINITY;
    let mut best_move: Option<Move> = None;
    // Fail-high means score is atleast this good so lower-bound/Fail-low means the score is an upper bound
    let mut bound = Bound::Upper;

    let mut move_picker = MovePicker::new(tt_move);
    let mut quiets_searched = StackVec::<Move, 32>::new();
    let mut noisies_searched = StackVec::<Move, 32>::new();
    let mut skip_quiets = false;

    while let Some(m) = move_picker.next(data, skip_quiets, ply) {
        if m == data.ply_table[ply].excluded {
            continue;
        }

        move_count += 1;
        let is_direct_check = data.board.is_direct_check(m);
        let is_quiet = m.get_kind().is_quiet();
        let history = if is_quiet {
            data.quiet_history.get(data.board.state.threats, stm, m)
                + data.get_conthistory(m, ply, 1)
                + data.get_conthistory(m, ply, 2)
        } else {
            let captured = data
                .board
                .get_piece_at_square(m.get_capture_square())
                .map(|(_, p)| p);
            data.noisy_history.get(
                data.board.get_piece_at_square(m.get_from()),
                m.get_to(),
                captured,
                data.board.state.threats,
            )
        };

        if !Node::ROOT && !mated(best_score) {
            // Late Move Pruning (LMP)
            if !in_check
                && !is_direct_check
                && !mating(beta)
                && is_quiet
                && move_count > (3 + depth as usize * depth as usize) / (2 - (improving as usize))
            {
                skip_quiets = true;
                continue;
            }

            // Futility Pruning (FP)
            if !in_check
                && !is_direct_check
                && is_quiet
                && depth < 7
                && static_eval + 90 * depth + 146 <= alpha
            {
                skip_quiets = true;
                continue;
            }

            // Bad Noisy Futility Pruning (BNFP)
            if !in_check
                && depth < 10
                && move_picker.stage() == Stage::BadNoisy
                && static_eval + 150 * depth <= alpha
            {
                break;
            }

            // History Pruning (HP)
            if !in_check && is_quiet && depth <= 5 && history < -1482 * depth {
                continue;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = (-123 * depth * depth - 47 * depth + 13).min(-33);
            if !in_check && !is_quiet && !data.board.see(m, threshold) {
                continue;
            }
        }

        let initial_nodes = data.nodes();

        // Make Move
        data.make_move(m, ply);
        let new_depth = (depth - 1) + (in_check as i32) + ((move_count == 1) as i32 * extension);
        let mut score = -Score::INFINITY;

        // Late Move Reductions (LMR)
        if depth > 2 && move_count > 1 {
            let mut r = LMR_TABLE[is_quiet as usize][depth.min(127) as usize][move_count.min(63)];
            r += 217 * !improving as i32;
            r -= 200 * tt_pv as i32;
            r += 450 * (tt_score.is_some_and(|s| s <= alpha)) as i32;
            r += 300 * (tt_depth.is_some_and(|d| d < depth)) as i32;

            let reduction = r / 1024;
            let reduced_depth = (new_depth - reduction).max(1) + Node::PV as i32;

            score = -search::<NonPV>(data, reduced_depth, -alpha - 1, -alpha, ply + 1, true);
            if score > alpha && reduced_depth < new_depth {
                score = -search::<NonPV>(data, new_depth, -alpha - 1, -alpha, ply + 1, !cutnode);
            }
        } else if !Node::PV || move_count > 1 {
            score = -search::<NonPV>(data, new_depth, -alpha - 1, -alpha, ply + 1, !cutnode);
        }

        // Principal Variation Search (PVS)
        if Node::PV && (move_count == 1 || score > alpha) {
            score = -search::<PV>(data, new_depth, -beta, -alpha, ply + 1, false);
        }

        // Unmake Move
        data.unmake_move();

        if data.shared.status.get() == Status::STOPPED {
            return Score::TIMEOUT;
        }

        if Node::ROOT {
            let nodes = data.nodes();
            if let Some(root_move) = data.root_moves.iter_mut().find(|rm| rm.m == m) {
                root_move.nodes += nodes - initial_nodes;
            };
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = Some(m);
                bound = Bound::Exact;
                data.pv.add(m, ply);

                // Cutoff
                if score >= beta {
                    bound = Bound::Lower;
                    break;
                }

                alpha = score;
            }
        }

        // Add searched quiet/noisy moves to list
        if best_move != Some(m) && move_count < 32 {
            if is_quiet {
                quiets_searched.push(m);
            } else {
                noisies_searched.push(m);
            }
        }
    }

    if move_count == 0 {
        if excluded {
            return -Score::INFINITY + 1;
        }

        if in_check {
            return -Score::MATE + ply as i32;
        } else {
            return Score::DRAW;
        }
    }

    if let Some(m) = best_move {
        let is_quiet = m.get_kind().is_quiet();

        let quiet_bonus = (319 * depth).min(928) - 227;
        let quiet_malus = (287 * depth).min(955) - 236;

        let noisy_bonus = (259 * depth).min(1060) - 198;
        let noisy_malus = (308 * depth).min(934) - 277;

        let cont_bonus = (308 * depth).min(1060) - 196;
        let cont_malus = (303 * depth).min(1081) - 270;

        let threats = data.board.state.threats;

        if is_quiet {
            // Add quiet bonus to history
            data.quiet_history.update(threats, stm, m, quiet_bonus);
            // Conthistory Bonus
            data.update_conthistories(m, ply, cont_bonus);
            // Add malus to quiet moves
            for e in quiets_searched.iter() {
                let quiet_move = e;
                data.quiet_history
                    .update(threats, stm, *quiet_move, -quiet_malus);

                // Conthistory malus
                data.update_conthistories(*quiet_move, ply, -cont_malus);
            }
        } else {
            // Add noisy bonus to history
            let piece = data.board.get_piece_at_square(m.get_from());
            let to = m.get_to();
            let captured = data
                .board
                .get_piece_at_square(m.get_capture_square())
                .map(|e| e.1);
            data.noisy_history
                .update(piece, to, captured, threats, noisy_bonus);
        }

        // Add malus to noisy moves
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

    if !excluded {
        data.shared.tt.add_entry(
            best_move.unwrap_or_default(),
            best_score,
            raw_eval,
            bound,
            data.board.state.keys.full,
            depth,
            ply,
            Node::PV,
        );
    }

    // Update Correction Histories
    if !in_check
        && best_move.is_none_or(|m| m.get_kind().is_quiet())
        && ((bound == Bound::Lower && best_score >= static_eval)
            || (bound == Bound::Upper && best_score <= static_eval)
            || bound == Bound::Exact)
    {
        data.update_correction_histories(best_score - static_eval, depth);
    }

    best_score
}

pub fn quiesce<Node: NodeType>(
    data: &mut SearchData,
    mut alpha: i32,
    beta: i32,
    ply: isize,
) -> i32 {
    data.shared.increment_nodes(data.id);
    if data.board.is_draw() {
        return Score::DRAW;
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

    let tt_entry = data.shared.tt.get_entry(data.board.state.keys.full, ply);

    // TT Cutoffs
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
            return Score::DRAW;
        } else {
            return data.network.evaluate(&data.board);
        }
    }

    // Evaluation
    let raw_eval;
    let static_eval;
    let mut best_score;

    if in_check {
        raw_eval = -Score::INFINITY;
        static_eval = -Score::INFINITY;
        best_score = static_eval;
    } else if let Some(e) = &tt_entry
        && e.get_eval() != -Score::INFINITY
    {
        raw_eval = e.get_eval();
        static_eval = raw_eval + data.correction();
        best_score = static_eval;
    } else {
        raw_eval = data.network.evaluate(&data.board);
        static_eval = raw_eval + data.correction();
        best_score = static_eval
    };

    // Stand Pat
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
            // Late Move Pruning (LMP)
            if move_count >= 3 {
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            if !data.board.see(m, -134) {
                continue;
            }
        }

        data.make_move(m, ply);
        let score = -quiesce::<Node>(data, -beta, -alpha, ply + 1);
        data.unmake_move();

        if data.shared.status.get() == Status::STOPPED {
            return Score::TIMEOUT;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = Some(m);

                // Cutoff
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
        // Add noisy bonus to history
        let piece = data.board.get_piece_at_square(m.get_from());
        let to = m.get_to();
        let captured = data
            .board
            .get_piece_at_square(m.get_capture_square())
            .map(|e| e.1);
        data.noisy_history
            .update(piece, to, captured, data.board.state.threats, 98);
    }

    data.shared.tt.add_entry(
        best_move.unwrap_or_default(),
        best_score,
        raw_eval,
        bound,
        data.board.state.keys.full,
        0,
        ply,
        Node::PV,
    );

    best_score
}
