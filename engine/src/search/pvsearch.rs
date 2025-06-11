use chess::types::{eval::Eval, moves::Move};

use crate::{
    history::movebuffer::MoveBuffer,
    position::{
        movepick::{MPStage, MovePicker, SearchType},
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
        // Base case: depth = 0.
        if depth <= 0 {
            return self.qsearch::<NT::Next>(t, tt, pv, alpha, beta);
        }

        pv.clear();

        t.seldepth = if NT::RT { 0 } else { t.seldepth.max(t.ply) };

        if !NT::RT {
            // Check for upcoming draw.
            if alpha < Eval::DRAW && self.board.upcoming_repetition(t.ply) {
                alpha = Eval::dithered_draw(t.nodes as i32);
                if alpha >= beta {
                    return alpha;
                }
            }

            // Check if we should stop here in the search.
            if t.should_stop() {
                t.stop = true;
                return Eval::DRAW;
            }

            // Mate distance pruning.
            // If we have already found a faster mate,
            // then we don't need to search this node.
            alpha = alpha.max(Eval::mated_in(t.ply));
            beta = beta.min(Eval::mate_in(t.ply + 1));

            if alpha >= beta {
                return alpha;
            }

            // Check for immediate draw.
            if self.board.is_draw(t.ply_from_null) {
                return Eval::dithered_draw(t.nodes as i32);
            }
        }

        let in_check = self.board.in_check();
        let excluded = t.ss().excluded;
        let singular = !excluded.is_null();
        let child_pv = &mut PVLine::default();
        let mut ext_possible = false;

        // -----------------------------------
        //             TT lookup
        // -----------------------------------
        let tt_entry = tt.probe(self.board.state.hash);
        let mut tt_move = Move::NONE;
        let mut tt_depth = -1;
        let mut tt_value = Eval::DRAW;

        if let Some(tte) = tt_entry {
            tt_depth = tte.depth();
            let tt_bound = tte.bound();

            tt_value = tte.value(t.ply);

            // TT cutoff.
            if !NT::PV
                && !singular
                && tt_depth >= depth
                && match tt_bound {
                    Bound::Lower => tt_value >= beta,
                    Bound::Upper => tt_value <= alpha,
                    Bound::Exact => true,
                    _ => false,
                }
            {
                return tt_value;
            }

            tt_move = tte.mov();

            ext_possible = !NT::RT && depth >= ext_d_min() && !tt_value.is_tb_mate() && tt_bound != Bound::Upper && tt_depth >= depth - 3;
        }

        if !singular {
            t.ss_mut().ttpv = NT::PV
        }

        // TODO: Tablebases probe.

        // -----------------------------------
        //            Static Eval
        // -----------------------------------
        let eval = if singular {
            t.ss().eval

        // Don't evaluate positions in check.
        } else if in_check {
            // Get the previous eval if possible.
            let prev_eval = if t.ply >= 2 { t.ss_at(2).eval } else { -Eval::INFINITY };
            t.ss_mut().eval = prev_eval;
            prev_eval

        // If we have already evaluated this position use that instead.
        } else if let Some(tte) = tt_entry {
            let e = if tte.eval() == -Eval::INFINITY { self.evaluate(&mut t.nnue) } else { tte.eval() };
            t.ss_mut().eval = e;

            // If the tt eval is a tighter bound than the static eval, use it instead.
            // Otherwise, just use static eval.
            tte.get_tightest(e, t.ply)

        // If nothing else, evaluate the position from scratch.
        } else {
            let nnue_eval = self.evaluate(&mut t.nnue);
            t.ss_mut().eval = nnue_eval;
            nnue_eval
        };

        let improving = t.is_improving(in_check);

        // -----------------------------------
        //              Pruning
        // -----------------------------------
        if !NT::PV && !in_check {
            // Reverse futility pruning (static null move pruning).
            if can_apply_rfp(t, depth, improving, eval, beta) {
                return beta + (eval - beta) / 3;
            }

            // Razoring.
            if can_apply_razoring(depth, eval, alpha) {
                return self.qsearch::<OffPV>(t, tt, pv, alpha, beta);
            }

            // Null move pruning.
            if !singular && can_apply_nmp(&self.board, t, depth, improving, eval, beta) {
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
        if can_apply_iir(depth, tt_move, NT::PV, cutnode) {
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

        let old_alpha = alpha;

        let mut mp = MovePicker::new(SearchType::Pv, in_check, tt_move);

        // Set up margins.
        let lmp_margin = (depth * depth + lmp_base()) as usize;
        let see_margin = [Eval(sp_noisy_margin() * (depth * depth) as i32), Eval(sp_quiet_margin() * depth as i32)];

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

            // -----------------------------------
            //            More Pruning
            // -----------------------------------
            // Futility pruning and late move pruning.
            if !NT::RT && is_quiet && !best_eval.is_loss() && !mp.skip_quiets && !in_check {
                mp.skip_quiets = can_apply_fp(depth, eval, alpha, moves_tried) || can_apply_lmp(depth, moves_tried, lmp_margin);
            }

            // SEE pruning.
            if !best_eval.is_loss()
                && depth <= sp_d_min()
                && mp.stage > MPStage::PvNoisyWin
                && !self.board.see(m, see_margin[is_quiet as usize])
            {
                moves_tried += 1;
                continue;
            }

            // -----------------------------------
            //             Extensions
            // -----------------------------------
            let mut new_depth = depth - 1;

            if ext_possible && m == tt_move {
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
                }
                // Multi-cut pruning.
                else if v >= beta && !v.is_tb_mate() {
                    return v;
                }
                // Negative extensions.
                else if tt_value >= beta {
                    ext = -3;
                }
                // Use negative extensions for cutnodes.
                else if cutnode {
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
            if can_apply_lmr(depth, moves_tried, NT::PV) {
                let mut r = lmr_base_reduction(depth, moves_tried);

                // Decrease reductions for good moves, increase reductions for bad moves.
                r -= t.ss().ttpv as i16;
                r -= in_check as i16;
                r -= (tt_depth >= depth) as i16;
                r -= (hist_score / (if is_quiet { hist_quiet_div() } else { hist_noisy_div() })) as i16;

                r += (!NT::PV as i16) * 2;
                r += (cutnode as i16) * 2;
                r += !improving as i16;
                r += tt_move.flag().is_noisy() as i16;

                // We shouldn't extend or drop into qsearch.
                r = r.max(0);

                v = -self.nwsearch(t, tt, child_pv, -alpha, new_depth - r, true);

                // Verification search.
                // If LMR search succeeds, then do a full search to verify it.
                if v > alpha && r > 0 {
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
                    alpha = best_eval;

                    if NT::PV {
                        pv.update(m, child_pv);
                    }
                }

                if v >= beta {
                    alpha = beta;
                    t.update_history(m, depth, &self.board, &quiets_tried, &caps_tried);
                    break;
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

        // Store the result in the TT.
        self.store_search_result(t, tt, best_eval, alpha, beta, old_alpha, best_move, depth, NT::PV);

        alpha
    }
}
