use chess::types::{eval::Eval, moves::Move};

use crate::{
    history::movebuffer::MoveBuffer,
    position::{
        movepick::{MovePicker, SearchType},
        pos::Pos,
    },
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
    tunables::params::tunables::*,
};

use super::{NodeType, OffPV, helpers::*, pv::PVLine};

impl Pos {
    /// Principal variation search function.
    /// This performs the majority of the searching, then drops into qsearch at the end.
    #[allow(clippy::too_many_arguments)]
    pub fn pvsearch<NT: NodeType>(
        &mut self,
        t: &mut Thread,
        tt: &TT,
        pv: &mut PVLine,
        mut alpha: Eval,
        mut beta: Eval,
        mut depth: i16,
        cutnode: bool,
    ) -> Eval {
        // Check if we should stop here in the search.
        if t.should_stop() {
            t.stop = true;
            return Eval::DRAW;
        }

        // Base case: depth = 0.
        if depth <= 0 {
            return self.qsearch::<NT::Next>(t, tt, alpha, beta);
        }

        if NT::PV {
            // Clear invalid moves from PV.
            pv.clear();

            // Update seldepth.
            // Seldepth counts from 1.
            t.seldepth = t.seldepth.max(t.ply + 1);
        }

        if !NT::RT {
            // Check for upcoming draw.
            if alpha < Eval::DRAW && self.board.upcoming_repetition(t.ply) {
                alpha = Eval::dithered_draw(t.nodes as i32);
                if alpha >= beta {
                    return alpha;
                }
            }

            // Check for immediate draw.
            if self.board.is_draw(t.ply_from_null) {
                return Eval::dithered_draw(t.nodes as i32);
            }

            // Mate distance pruning.
            // If we have already found a faster mate,
            // then we don't need to search this node.
            alpha = alpha.max(Eval::mated_in(t.ply));
            beta = beta.min(Eval::mate_in(t.ply + 1));

            if alpha >= beta {
                return alpha;
            }
        }

        let in_check = self.board.in_check();
        let excluded = t.ss().excluded;
        let singular = excluded.is_valid();
        let child_pv = &mut PVLine::default();

        // -----------------------------------
        //             TT lookup
        // -----------------------------------
        let mut tt_move = Move::NONE;
        let mut tt_eval = Eval::NONE;
        let mut tt_value = Eval::NONE;
        let mut tt_bound = Bound::None;
        let mut tt_pv = NT::PV;
        let mut tt_depth = -1;

        if let Some(tte) = tt.probe(self.board.state.hash) {
            tt_move = tte.mov();
            tt_eval = tte.eval();
            tt_value = tte.value(t.ply);
            tt_bound = tte.bound();
            tt_depth = tte.depth();

            tt_pv |= tte.pv();
        };

        // TT cutoff.
        // In a non-pv node, if the TT lookup gives us a better position evaluation, use it
        // instead.
        if !NT::PV && !singular && tt_value.is_valid() && tt_depth >= depth && tt_bound.is_usable(tt_value, beta) {
            return tt_value;
        }

        // TODO: Tablebases probe.

        // -----------------------------------
        //            Static Eval
        // -----------------------------------
        let mut eval;
        let mut improving = false;

        // Don't evaluate positions in check.
        // We will also skip all the pruning if we are in check as well.
        if in_check {
            let v = if t.ply >= 2 { t.ss_at(2).eval } else { Eval::NONE };
            t.ss_mut().eval = v;
            eval = v
        } else {
            // In singular search, just take the eval of the position that invoked it.
            if singular {
                eval = t.ss().eval;

            // Otherwise try to get the eval from the tt.
            } else {
                eval = if tt_eval.is_valid() { tt_eval } else { self.evaluate(&mut t.nnue) };

                t.ss_mut().eval = eval;

                if tt_value.is_valid() && tt_bound.is_usable(tt_value, eval) {
                    eval = tt_value
                }
            }

            improving = t.is_improving();
        }

        // -----------------------------------
        //              Pruning
        // -----------------------------------
        if !NT::PV && !in_check && !singular {
            // Reverse futility pruning (static null move pruning).
            if can_apply_rfp(depth, improving, eval, beta) {
                return beta;
            }

            // Razoring.
            if can_apply_razoring(depth, eval, alpha) {
                let v = self.qsearch::<OffPV>(t, tt, alpha, beta);
                // If the qsearch still can't catch up, cut this node.
                if v <= alpha {
                    return v;
                }
            }

            // Null move pruning.
            if can_apply_nmp(&self.board, t, depth, improving, eval, beta) {
                let r = (nmp_base() + depth / nmp_factor()).min(depth);

                self.make_null(t);
                let v = -self.nwsearch(t, tt, child_pv, -(beta - Eval(1)), depth - r, false);
                self.undo_null(t);

                if v >= beta && !v.is_tb_mate() {
                    return beta;
                }
            }
        }

        // Internal Iterative reductions.
        if !in_check && can_apply_iir(depth, tt_move, NT::PV, cutnode) {
            depth -= 1;
        }

        // TODO: Probcut.

        // -----------------------------------
        //             Moves loop
        // -----------------------------------
        let mut best_eval = -Eval::INFINITY;
        let mut best_move = Move::NONE;

        let mut caps_tried = MoveBuffer::default();
        let mut quiets_tried = MoveBuffer::default();
        let mut moves_tried = 0;

        let lmp_margin = ((depth * depth + lmp_base()) / (2 - improving as i16)) as usize;

        let mut mp = MovePicker::new(SearchType::Pv, in_check, tt_move);

        while let Some(m) = mp.next(&self.board, t) {
            debug_assert!(m.is_valid());

            // Ignore excluded move.
            if m == excluded {
                continue;
            }

            moves_tried += 1;

            let start_nodes = t.nodes;
            let is_quiet = m.flag().is_quiet();
            let hist_score = t.hist_score(&self.board, m);
            let mut new_depth = depth - 1;

            // -----------------------------------
            //          Move loop pruning
            // -----------------------------------
            if !NT::RT && !best_eval.is_loss() && !self.board.only_king_pawns_left() {
                let mut r = lmr_base_reduction(depth, moves_tried);
                r += !improving as i16;
                r -= (hist_score / mlp_hist_div()) as i16;

                let d = (depth - r).max(0);

                let threshold = if is_quiet { sp_quiet_margin() * (d * d) as i32 } else { sp_noisy_margin() * depth as i32 };

                // SEE pruning.
                if !self.board.see(m, Eval(threshold)) {
                    continue;
                }

                // History pruning.
                if is_quiet && hist_score < hp_hist_d_mult() * depth as i32 {
                    mp.skip_quiets = true;
                    continue;
                }

                // Late move pruning.
                // If we have seen a lot of moves in this position already, and we don't expect something good
                // from this move, then we should skip the quiet moves.
                if can_apply_lmp(depth, moves_tried, lmp_margin) {
                    mp.skip_quiets = true;
                }

                // Futility pruning.
                if can_apply_fp(depth, eval, alpha, moves_tried) {
                    mp.skip_quiets = true;
                    continue;
                }
            }

            // -----------------------------------
            //             Extensions
            // -----------------------------------
            // If all moves but one fail low but one, then that move is a singular move.
            // This is verified with a search on half depth on that move.
            if !NT::RT
                && !singular
                && depth >= 5
                && m == tt_move
                && !tt_value.is_tb_mate()
                && tt_bound.has(Bound::Lower)
                && tt_depth >= depth - 3
            {
                let ext_beta = (tt_value - (depth * ext_mult() / 64)).max(-Eval::INFINITY);

                t.ss_mut().excluded = m;
                let v = self.nwsearch(t, tt, pv, ext_beta, new_depth / 2, cutnode);
                t.ss_mut().excluded = Move::NULL;

                let mut ext = 0;

                // Single and double extensions.
                if v < ext_beta {
                    ext = 1
                        + (!NT::PV && v < ext_beta - ext_double_e_min()) as i16
                        + (!NT::PV && is_quiet && v < ext_beta - ext_triple_e_min()) as i16;

                // Multi-cut pruning.
                } else if ext_beta >= beta {
                    return ext_beta;

                // Negative extensions.
                } else if tt_value >= beta {
                    ext = -2 + NT::PV as i16;

                // Use negative extensions for cutnodes.
                } else if cutnode {
                    ext = -2
                }

                new_depth += ext;
            }

            // -----------------------------------
            //             Make Move
            // -----------------------------------
            self.make_move(m, t);
            tt.prefetch(self.board.state.hash);

            let mut v = -Eval::INFINITY;

            // Late move reductions.
            // If we have searched enough moves so that we should start
            // reducing our search depth, then we should start with this search.
            if can_apply_lmr(depth, moves_tried, NT::RT) {
                let mut r = lmr_base_reduction(depth, moves_tried);
                // Decrease reductions for good moves, increase reductions for bad moves.
                r -= (hist_score / (if is_quiet { hist_quiet_div() } else { hist_noisy_div() })) as i16;
                r -= self.board.in_check() as i16;
                r -= in_check as i16;
                r -= (tt_depth >= depth) as i16;
                r -= tt_pv as i16;

                r += !NT::PV as i16;
                r += !improving as i16;
                r += tt_move.flag().is_noisy() as i16;
                r += cutnode as i16;

                // We shouldn't extend or drop into qsearch.
                r = r.clamp(1, depth - 1);

                v = -self.nwsearch(t, tt, child_pv, -alpha, new_depth - r, true);

                // Verification search.
                // If LMR search succeeds, then do a full search to verify it.
                if v > alpha && r > 1 {
                    new_depth += (v > best_eval + lmr_ver_e_min()) as i16;
                    new_depth -= (v < best_eval + new_depth && !NT::RT) as i16;

                    v = -self.nwsearch(t, tt, child_pv, -alpha, new_depth, !cutnode);
                }
            }
            // If we can't do LMR, then instead do a null window search at full depth.
            else if !NT::PV || moves_tried > 1 {
                v = -self.nwsearch(t, tt, child_pv, -alpha, new_depth, !cutnode);
            }

            // For the first move of each node, do a full depth, full window search.
            // We should also do that if the score breaks the upper bound.
            if NT::PV && (moves_tried == 1 || v > alpha) {
                v = -self.pvsearch::<NT::Next>(t, tt, child_pv, -beta, -alpha, new_depth, false);
            }

            self.undo_move(m, t);

            if t.stop {
                return Eval::DRAW;
            }

            if NT::RT {
                t.clock.update_node_count(m, t.nodes - start_nodes);
            }

            if v > best_eval {
                best_eval = v;

                if v > alpha {
                    best_move = m;

                    if NT::PV {
                        pv.update(m, child_pv);
                    }

                    if best_eval >= beta {
                        break;
                    }

                    alpha = best_eval;
                }
            }

            // Add move to history
            if is_quiet {
                quiets_tried.push(m);
            } else if m.flag().is_cap() {
                caps_tried.push(m);
            }
        }

        // No moves tried: check or checkmate.
        if moves_tried == 0 {
            return if in_check { Eval::mated_in(t.ply) } else { Eval::DRAW };
        }

        // Update histories.
        if best_eval >= beta {
            t.update_history(best_move, depth, &self.board, &quiets_tried, &caps_tried);
        }

        // Store the result in the TT.
        let result_bound = if best_eval >= beta {
            Bound::Lower
        } else if NT::PV && best_move.is_valid() {
            Bound::Exact
        } else {
            Bound::Upper
        };

        tt.insert(self.board.state.hash, result_bound, best_move, eval, best_eval, depth, t.ply, tt_pv);

        best_eval
    }
}
