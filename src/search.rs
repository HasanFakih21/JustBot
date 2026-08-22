use crate::search::data::{Report, SearchData, Status};
use crate::search::movepicker::MovePicker;
use crate::types::stack::Stack;
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

    let mut delta = 25;
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
        data.stack = Stack::new();

        if (data.time.hard_limit(data.nodes(), data.id)
            || data
                .time
                .node_limit()
                .is_some_and(|node_limit| data.nodes() >= node_limit)
            || depth > data.time.depth_limit())
            && data.id == 0
        {
            data.shared.status.stop();
            break;
        }

        let score = search::<Root>(data, depth, alpha, beta, 0, false);

        if data.shared.status.get() == Status::STOPPED {
            break;
        }

        // Aspiration Window
        if score <= alpha {
            // Failed Low
            alpha = (score - delta).max(-Score::INFINITY);
            beta = (alpha + delta).min(beta);
            delta += 25 * delta / 128;
            continue;
        } else if score >= beta {
            // Failed High
            alpha = (beta - delta).max(alpha);
            beta = (score + delta).min(Score::INFINITY);
            delta += 25 * delta / 128;
            continue;
        }

        depth += 1;
        data.root_moves
            .sort_by_key(|rm| std::cmp::Reverse(rm.score));
        best_move = Some(data.root_moves[0].m);

        if data.report == Report::Full {
            data.print_uci_info(depth);
        }

        let multiplier = || {
            let ratio = data.root_moves[0].nodes as f32 / data.nodes() as f32;
            (2.977 - ratio * 2.495).max(0.553)
        };

        if data.time.soft_limit(multiplier) && data.id == 0 {
            data.shared.status.stop();
            break;
        }

        delta = 25;
        alpha = (score - delta).max(-Score::INFINITY);
        beta = (score + delta).min(Score::INFINITY);
    }

    if data.report == Report::Minimal {
        data.print_uci_info(depth);
    }

    data.best_move = best_move;
}

pub fn search<Node: NodeType>(
    data: &mut SearchData,
    depth: i32,
    mut alpha: i32,
    mut beta: i32,
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
    let excluded = !data.stack[ply].excluded.is_null();

    if !Node::ROOT {
        // Check for draws
        if data.board.is_draw() {
            return Score::DRAW;
        }

        // Prevent from going too deep
        if ply >= MAX_PLY as isize - 1 {
            if in_check {
                return Score::DRAW;
            } else {
                return data.network.evaluate(&data.board);
            }
        }

        // Mate Distance Pruning (MDP)
        alpha = alpha.max(-Score::MATE + ply as i32);
        beta = beta.min(Score::MATE - ply as i32 + 1);
        if alpha >= beta {
            return alpha;
        }
    }

    // Check for Time Outs
    if (data.time.hard_limit(data.nodes(), data.id)
        || data
            .time
            .node_limit()
            .is_some_and(|node_limit| data.nodes() >= node_limit))
        && data.id == 0
    {
        data.shared.status.stop();
        return Score::TIMEOUT;
    }

    let mut depth = depth.min(MAX_PLY as i32 - 1);

    // Transposition Table Entries
    let tt_entry = data.shared.tt.entry(data.board.hash(), ply);
    let tt_move = tt_entry
        .as_ref()
        .map(|e| e.best_move())
        .filter(|m| !m.is_null());
    let tt_bound = tt_entry.as_ref().map(|e| e.bound());
    let tt_score = tt_entry
        .as_ref()
        .map(|e| e.score())
        .filter(|s| *s != Score::NONE);
    let tt_pv = tt_entry.as_ref().map(|e| e.is_pv()).unwrap_or(false);
    let tt_depth = tt_entry.as_ref().map(|e| e.depth());

    // TT Cutoffs
    if !Node::PV
        && let Some(tt_score) = tt_score
        && tt_depth.is_some_and(|d| d >= depth)
        && (tt_score <= alpha || cutnode)
        && !excluded
        && tt_bound.is_some_and(|b| match b {
            Bound::Lower => tt_score >= beta,
            Bound::Upper => tt_score < alpha,
            Bound::Exact => true,
            Bound::None => false,
        })
    {
        return tt_score;
    }

    // Evaluation
    let raw_eval;
    let static_eval;
    let correction = data.correction(ply);

    if in_check {
        raw_eval = Score::NONE;
        static_eval = Score::NONE;
    } else if excluded {
        raw_eval = Score::NONE;
        static_eval = data.stack[ply].eval
    } else if let Some(e) = &tt_entry
        && e.eval() != Score::NONE
    {
        raw_eval = e.eval();
        static_eval = raw_eval + correction;
    } else {
        raw_eval = data.network.evaluate(&data.board);
        static_eval = raw_eval + correction;
    };

    data.stack[ply].eval = static_eval;
    if !excluded && tt_entry.is_none() {
        data.shared.tt.add_entry(
            Move::default(),
            Score::NONE,
            raw_eval,
            Bound::None,
            data.board.hash(),
            0,
            ply,
            Node::PV,
        );
    }

    let improvement = if in_check {
        0
    } else if data.stack[ply - 2].eval != Score::NONE {
        static_eval - data.stack[ply - 2].eval
    } else if data.stack[ply - 4].eval != Score::NONE {
        static_eval - data.stack[ply - 4].eval
    } else {
        0
    };

    let improving = improvement > 0;

    // Hindsight Reduction
    if !Node::ROOT
        && !in_check
        && !excluded
        && depth >= 2
        && data.stack[ply - 1].eval != Score::NONE
        && data.stack[ply - 1].reduction >= 2048
        && static_eval + data.stack[ply - 1].eval >= 200
    {
        depth -= 1;
    }

    // Razoring
    if !Node::PV
        && !in_check
        && tt_bound.is_none_or(|b| b != Bound::Lower)
        && static_eval < alpha - 246 - 253 * depth * depth
        && alpha < 2000
    {
        return quiesce::<Node>(data, alpha, beta, ply);
    }

    // Reverse Futillity Pruning (RFP)
    if !in_check
        && !Node::PV
        && !excluded
        && static_eval >= beta + 85 * depth + 5 * depth * depth - 75 * improving as i32
        && !is_decisive(beta)
        && !is_decisive(static_eval)
    {
        return ilerp::<1024>(static_eval, beta, 690);
    }

    // Null Move Pruning
    if cutnode
        && depth >= 3
        && !excluded
        && !in_check
        && !data.board.only_king_and_pawns()
        && tt_bound.is_none_or(|b| b != Bound::Upper)
        && static_eval >= beta + (200 - 1250 * depth / 128 - 63 * improving as i32).max(0)
        && !data.stack[ply - 1].m.is_null()
    {
        let r = 6 + depth * 132 / 637;
        data.stack[ply].conthistory = data.stack.sentinel();
        data.stack[ply].contcorrhistory = data.stack.sentinel();
        data.stack[ply].m = Move::default();
        data.stack[ply].piece = OptionPiece::None;

        data.board.make_null_move();
        data.shared.tt.prefetch(data.board.hash());

        let null_move_score = -search::<NonPV>(data, depth - r, -beta, -beta + 1, ply + 1, false);
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
        && depth >= 5
        && tt_depth.is_some_and(|d| d >= depth - 3)
        && let Some(tt_move) = tt_move
        && let Some(tt_bound) = tt_bound
        && let Some(tt_score) = tt_score
        && !is_decisive(tt_score)
        && tt_bound != Bound::Upper
    {
        let singular_depth = (depth - 1) / 2;
        let singular_beta = tt_score - (depth + depth);

        data.stack[ply].excluded = tt_move;
        data.stack[ply].m = Move::default();
        // Search everything except the TT move with a null window at a reduced depth to find out if it's worth extending or not
        let singular_score = search::<NonPV>(
            data,
            singular_depth,
            singular_beta - 1,
            singular_beta,
            ply,
            cutnode,
        );
        data.stack[ply].excluded = Move::default();

        if data.shared.status.get() == Status::STOPPED {
            return Score::TIMEOUT;
        }

        if singular_score < singular_beta {
            let double_margin = 10 + 150 * Node::PV as i32 + 50 * (Node::PV && !tt_pv) as i32;
            let triple_margin = 100 + 350 * Node::PV as i32 + 50 * (Node::PV && !tt_pv) as i32;
            extension = 1
                + (singular_score < singular_beta - double_margin) as i32
                + (singular_score < singular_beta - triple_margin) as i32;
        }
        // Negative Extensions
        else if tt_score >= beta || cutnode {
            extension -= 3;
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
        if m == data.stack[ply].excluded {
            continue;
        }

        move_count += 1;
        let is_direct_check = data.board.is_direct_check(m);
        let is_quiet = m.kind().is_quiet();
        let history = if is_quiet {
            data.quiet_history.get(data.board.state.threats, stm, m)
                + data.conthistory(m, ply, 1)
                + data.conthistory(m, ply, 2)
        } else {
            let captured = data
                .board
                .piece_at_square(m.capture_square())
                .map(|p| p.kind());
            data.noisy_history.get(
                data.board.piece_at_square(m.from()),
                m.to(),
                captured,
                data.board.state.threats,
            )
        };

        if !Node::ROOT && !is_loss(best_score) {
            // Late Move Pruning (LMP)
            if !in_check
                && !is_direct_check
                && !is_win(beta)
                && is_quiet
                && move_count as i32 > (3011 + 1493 * depth * depth) / 1024
            {
                skip_quiets = true;
                continue;
            }

            // Futility Pruning (FP)
            if !in_check
                && !is_direct_check
                && is_quiet
                && depth < 8
                && static_eval + 93 * depth + 146 + 50 * history / 1024 <= alpha
            {
                skip_quiets = true;
                continue;
            }

            // History Pruning (HP)
            if !in_check && is_quiet && depth <= 6 && history < -1485 * depth {
                continue;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = (-125 * depth * depth - 46 * depth + 14).min(-34);
            if !in_check && !is_quiet && !data.board.see(m, threshold) {
                continue;
            }
        }

        let initial_nodes = data.nodes();

        // Make Move
        data.make_move(m, ply);
        let new_depth = (depth - 1) + ((move_count == 1) as i32 * extension);
        let mut score = -Score::INFINITY;

        // Late Move Reductions (LMR)
        if depth > 2 && move_count > 1 {
            let mut r = LMR_TABLE[is_quiet as usize][depth.min(127) as usize][move_count.min(63)];
            r += 217 * !improving as i32;
            r -= 197 * tt_pv as i32;
            r += 447 * (tt_score.is_some_and(|s| s <= alpha)) as i32;
            r += 296 * (tt_depth.is_some_and(|d| d < depth)) as i32;
            r -= 449 * history / 4096;

            let reduction = r / 1024;
            let reduced_depth = (new_depth - reduction).max(1) + Node::PV as i32;

            data.stack[ply].reduction = r;
            score = -search::<NonPV>(data, reduced_depth, -alpha - 1, -alpha, ply + 1, true);
            data.stack[ply].reduction = 0;

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

                if move_count == 1 || score > alpha {
                    root_move.score = score;
                    root_move.pv.commit(&data.pv.inner[1][..data.pv.len[1]]);
                } else {
                    root_move.score = -Score::INFINITY;
                }
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
            return -Score::INFINITY;
        }

        if in_check {
            return -Score::MATE + ply as i32;
        } else {
            return Score::DRAW;
        }
    }

    if let Some(m) = best_move {
        let is_quiet = m.kind().is_quiet();

        let quiet_bonus = (321 * depth).min(935) - 228;
        let quiet_malus = (289 * depth).min(948) - 232;

        let noisy_bonus = (257 * depth).min(1058) - 196;
        let noisy_malus = (302 * depth).min(937) - 273;

        let cont_bonus = (315 * depth).min(1044) - 194;
        let cont_malus = (303 * depth).min(1079) - 271;

        let threats = data.board.state.threats;

        if is_quiet {
            let piece = data.board.piece_at_square(m.from());
            let to = m.to();
            let pawn_key = data.board.state.keys.pawn;
            // Pawn History Bonus
            data.pawn_history.update(pawn_key, piece, to, quiet_bonus);
            // Quiet History Bonus
            data.quiet_history.update(threats, stm, m, quiet_bonus);
            // Conthistory Bonus
            data.update_conthistories(m, ply, cont_bonus);
            for quiet_move in quiets_searched.iter() {
                let piece = data.board.piece_at_square(quiet_move.from());
                let to = quiet_move.to();
                // Pawn History Malus
                data.pawn_history.update(pawn_key, piece, to, -quiet_malus);
                // Quiet History Malus
                data.quiet_history
                    .update(threats, stm, *quiet_move, -quiet_malus);
                // Conthistory Malus
                data.update_conthistories(*quiet_move, ply, -cont_malus);
            }
        } else {
            // Noisy History Bonus
            let piece = data.board.piece_at_square(m.from());
            let to = m.to();
            let captured = data
                .board
                .piece_at_square(m.capture_square())
                .map(|e| e.kind());
            data.noisy_history
                .update(piece, to, captured, threats, noisy_bonus);
        }

        // Noisy History Malus
        for m in noisies_searched.iter() {
            let piece = data.board.piece_at_square(m.from());
            let to = m.to();
            let captured = data
                .board
                .piece_at_square(m.capture_square())
                .map(|e| e.kind());
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
            data.board.hash(),
            depth,
            ply,
            Node::PV,
        );
    }

    // Update Correction Histories
    if !in_check
        && best_move.is_none_or(|m| m.kind().is_quiet())
        && ((bound == Bound::Lower && best_score >= static_eval)
            || (bound == Bound::Upper && best_score <= static_eval)
            || bound == Bound::Exact)
    {
        data.update_correction_histories(best_score - static_eval, depth, ply);
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

    if (data.time.hard_limit(data.nodes(), data.id)
        || data
            .time
            .node_limit()
            .is_some_and(|node_limit| data.nodes() >= node_limit))
        && data.id == 0
    {
        data.shared.status.stop();
        return Score::TIMEOUT;
    }

    let tt_entry = data.shared.tt.entry(data.board.hash(), ply);
    let tt_bound = tt_entry.as_ref().map(|e| e.bound());
    let tt_score = tt_entry
        .as_ref()
        .map(|e| e.score())
        .filter(|s| *s != Score::NONE);

    // TT Cutoffs
    if !Node::PV
        && let Some(tt_score) = tt_score
        && tt_bound.is_some_and(|b| match b {
            Bound::Lower => tt_score >= beta,
            Bound::Upper => tt_score < alpha,
            Bound::Exact => true,
            Bound::None => false,
        })
    {
        return tt_score;
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
        raw_eval = Score::NONE;
        static_eval = -Score::INFINITY;
        best_score = static_eval;
    } else if let Some(e) = &tt_entry
        && e.eval() != Score::NONE
    {
        raw_eval = e.eval();
        static_eval = raw_eval + data.correction(ply);
        best_score = static_eval;
    } else {
        raw_eval = data.network.evaluate(&data.board);
        static_eval = raw_eval + data.correction(ply);
        best_score = static_eval
    };

    if tt_entry.is_none() {
        data.shared.tt.add_entry(
            Move::default(),
            Score::NONE,
            raw_eval,
            Bound::None,
            data.board.hash(),
            0,
            ply,
            Node::PV,
        );
    }

    // Stand Pat
    if best_score >= beta {
        return best_score;
    }

    if best_score > alpha {
        alpha = best_score;
    }

    let tt_move = tt_entry.map(|e| e.best_move()).filter(|m| !m.is_null());

    let mut move_picker = MovePicker::new(tt_move);
    let mut move_count = 0;
    let mut bound = Bound::Upper;
    let mut best_move: Option<Move> = None;
    let skip_quiets = !in_check;

    while let Some(m) = move_picker.next(data, skip_quiets, ply) {
        move_count += 1;

        if !is_loss(best_score) {
            // Late Move Pruning (LMP)
            if move_count >= 3 && !data.board.is_direct_check(m) {
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            if !data.board.see(m, -129) {
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
        && !m.kind().is_quiet()
    {
        // Add noisy bonus to history
        let piece = data.board.piece_at_square(m.from());
        let to = m.to();
        let captured = data
            .board
            .piece_at_square(m.capture_square())
            .map(|e| e.kind());
        data.noisy_history
            .update(piece, to, captured, data.board.state.threats, 103);
    }

    data.shared.tt.add_entry(
        best_move.unwrap_or_default(),
        best_score,
        raw_eval,
        bound,
        data.board.hash(),
        0,
        ply,
        Node::PV,
    );

    best_score
}
