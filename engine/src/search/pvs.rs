use chess::{
    MAX_DEPTH,
    types::{eval::Eval, moves::Move},
};

use crate::{
    position::{
        movepick::{MovePicker, SearchType},
        pos::Pos,
    },
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
    tunables::params::tunables::*,
};

use super::{NodeType, helpers::*, pv::PVLine};

impl Pos {
    /// Principal variation search function.
    /// This performs the majority of the searching, then drops into qsearch at the end.
    #[allow(clippy::too_many_arguments)]
    pub fn pvs<NT: NodeType>(
        &mut self,
        t: &mut Thread,
        tt: &TT,
        pv: &mut PVLine,
        mut alpha: Eval,
        mut beta: Eval,
        mut depth: i16,
        cutnode: bool,
    ) -> Eval {
        if t.should_stop() {
            t.stop = true;
            return Eval::DRAW;
        }

        let in_check = self.board.in_check();

        // Base case: depth = 0.
        if depth <= 0 && !in_check {
            return self.qsearch::<NT::Next>(t, tt, pv, alpha, beta);
        }

        // Other base case: If we are close to reaching max ply, quit here.
        if t.ply >= MAX_DEPTH - 4 {
            return if in_check { Eval::DRAW } else { self.evaluate(&mut t.nnue) };
        }

        // Check extensions
        if in_check && depth < MAX_DEPTH as i16 {
            depth += 1;
        }

        pv.clear();

        t.seldepth = if NT::RT { 0 } else { t.seldepth.max(t.ply) };

        if !NT::RT {
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

        let excluded = t.ss().excluded;
        let singular = !excluded.is_null();
        let mut ext_possible = false;

        // -----------------------------------
        //             TT lookup
        // -----------------------------------
        let tt_entry = tt.probe(self.board.state.hash);
        let mut tt_move = Move::NONE;
        let mut tt_depth = -1;

        if let Some(tte) = tt_entry {
            tt_depth = tte.depth();
            let tt_bound = tte.bound();
            let tt_value = tte.value(t.ply);

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

        let eval = if singular { t.ss().eval } else { self.static_eval(t, tt_entry, in_check) };
        let improving = t.is_improving(in_check);

        // Pruning
        if !NT::PV && !in_check && !singular {
            // Reverse futility pruning (static null move pruning).
            if can_apply_rfp(t, depth, improving, eval, beta) {
                return beta + (eval - beta) / 3;
            }

            // Null move pruning.
        }

        // TODO: Probcut?

        // -----------------------------------
        //             Moves loop
        // -----------------------------------
        let mut best_eval = -Eval::INFINITY;
        let mut best_move = Move::NONE;

        let mut caps_tried = Vec::with_capacity(32);
        let mut quiets_tried = Vec::with_capacity(32);
        let mut moves_tried = 0;

        let child_pv = &mut PVLine::default();

        let old_alpha = alpha;

        let mut mp = MovePicker::new(SearchType::Pv, in_check, tt_move);

        while let Some(m) = mp.next(&self.board, t) {
            assert!(m.is_valid());

            // Ignore excluded move.
            if m == excluded {
                continue;
            }

            moves_tried += 1;

            let start_nodes = t.nodes;
            let is_quiet = m.flag().is_quiet();

            let mut new_depth = depth - 1;

            // Extensions.
            if ext_possible && m == tt_move {
                let tt_value = tt_entry.unwrap().value(t.ply);
                let ext_beta = (tt_value - (depth * ext_mult() / 64)).max(-Eval::INFINITY);

                t.ss_mut().excluded = m;
                let v = self.null_window_search(t, tt, pv, ext_beta, new_depth / 2, cutnode);
                t.ss_mut().excluded = Move::NULL;

                let mut ext = 0;

                // Single and double extensions.
                if v < ext_beta {
                    ext = if !NT::PV && v < ext_beta - ext_double_e_min() {
                        2 + (is_quiet && v < ext_beta - ext_triple_e_min()) as i16
                    } else {
                        1
                    }
                }
                // Multi-cut pruning.
                else if ext_beta >= beta {
                    return ext_beta;
                }
                // Negative extensions.
                else if tt_value >= beta {
                    ext = -2 + NT::PV as i16;
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
                let mut r = lmr_reduction(depth, moves_tried);

                // Reduce good moves less.
                r -= in_check as i16;
                r -= t.ss().ttpv as i16;
                r -= (tt_depth >= depth) as i16;

                // Reduce bad moves more.
                r += !NT::PV as i16;
                r += !improving as i16;
                r += cutnode as i16 * (2 - t.ss().ttpv as i16);

                // We shouldn't extend or drop into qsearch.
                let lmr_depth = (new_depth - r).clamp(1, new_depth + 1);

                v = -self.null_window_search(t, tt, child_pv, -alpha, lmr_depth, true);

                // Verification search.
                // If LMR search succeeds, then do a full search to verify it.
                if v > alpha && lmr_depth < new_depth {
                    new_depth += (v > best_eval + lmr_ver_e_min()) as i16;
                    new_depth -= (v < best_eval + new_depth && !NT::RT) as i16;

                    if lmr_depth < new_depth {
                        v = -self.null_window_search(t, tt, child_pv, -alpha, new_depth, !cutnode);
                    }
                }
            }
            // If we can't do LMR, then instead do a null window search at full depth.
            else if !NT::PV || moves_tried > 1 {
                v = -self.null_window_search(t, tt, child_pv, -alpha, new_depth, !cutnode);
            }

            // For the first move of each node, do a full depth, full window search.
            // We should also do that if the score breaks the upper bound.
            if NT::PV && (moves_tried == 1 || v > alpha) {
                v = -self.pvs::<NT::Next>(t, tt, child_pv, -beta, -alpha, new_depth, false);
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
                    t.update_history(m, depth, &self.board, quiets_tried, caps_tried);
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
