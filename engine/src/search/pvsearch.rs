use chess::{
    Depth, MAX_PLY,
    types::{eval::Eval, moves::Move},
};

use crate::{
    history::movebuffer::MoveBuffer,
    movepick::{MPStage, MovePicker, SearchType},
    position::Position,
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
    tunables::params::tunables::*,
};

use super::{NodeType, OffPV, pruning::*, pv::PVLine};

impl Position {
    /// Null window search.
    pub fn nwsearch(&mut self, t: &mut Thread, tt: &TT, pv: &mut PVLine, value: Eval, depth: Depth, cutnode: bool) -> Eval {
        self.pvsearch::<OffPV>(t, tt, pv, value - 1, value, depth, cutnode)
    }

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
        mut depth: Depth,
        cutnode: bool,
    ) -> Eval {
        // Base case: depth = 0.
        if depth <= 0 {
            return self.qsearch::<NT::Next>(t, tt, alpha, beta);
        }

        // Make sure we don't search too deep if extensions are going crazy.
        depth = depth.min((MAX_PLY - 1) as Depth);

        // Check if we should stop here in the search.
        if t.should_stop() {
            t.stop = true;
            return Eval::DRAW;
        }

        if NT::PV {
            // Clear invalid moves from PV.
            pv.clear();

            // Update seldepth.
            // Seldepth counts from 1.
            t.seldepth = t.seldepth.max(t.ply + 1);
        }

        // Initialize search node.
        let in_check = self.board.in_check();
        let excluded = t.ss().excluded;
        let singular = excluded.is_some();

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

            // Check if we are searching too deep.
            if t.ply >= MAX_PLY {
                return if in_check { Eval::dithered_draw(t.nodes as i32) } else { self.evaluate() };
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

        // -----------------------------------
        //             TT lookup
        // -----------------------------------

        let mut tt_move = Move::NONE;
        let mut tt_eval = -Eval::INFINITY;
        let mut tt_value = -Eval::INFINITY;
        let mut tt_bound = Bound::None;
        let mut tt_depth = -1;
        let mut tt_pv = NT::PV;

        if let Some(tte) = tt.probe(self.board.state.hash) {
            tt_move = tte.mov();
            tt_eval = tte.eval();
            tt_value = tte.value(t.ply);
            tt_bound = tte.bound();
            tt_depth = tte.depth();
            tt_pv |= tte.pv;
        }

        // TT cutoff.
        // In a non-PV node, if the TT lookup gives us a better position evaluation, use it instead.
        let tt_cutoff_d = depth - (tt_value <= beta) as Depth;
        if !NT::PV && !singular && tt_value.is_valid() && tt_depth >= tt_cutoff_d && tt_bound.is_usable(tt_value, beta) {
            return tt_value;
        }

        // TODO: Tablebases probe.

        // -----------------------------------
        //            Static Eval
        // -----------------------------------
        let mut raw_value = -Eval::INFINITY;

        // Don't evaluate positions in check.
        let eval = if in_check {
            t.ss_mut().eval = -Eval::INFINITY;
            -Eval::INFINITY
        }
        // In singular search, just take the eval of the position that invoked it.
        else if singular {
            raw_value = t.ss().eval;
            t.ss().eval
        }
        // Otherwise try to get eval from the tt if the position has been evaluated and the bound
        // is tighter. If we can't do that, then just evaluate the position from scratch.
        else {
            raw_value = if tt_eval.is_valid() { tt_eval } else { self.evaluate() };

            let mut e = self.adjust_eval(t, raw_value);
            t.ss_mut().eval = e;

            // If we have a TT hit with a tighter bound than our static eval, use the TT value.
            if tt_value.is_valid() && tt_bound.is_usable(tt_value, e) {
                e = tt_value
            }

            e
        };

        let improving = t.is_improving();
        let child_pv = &mut PVLine::default();

        // -----------------------------------
        //              Pruning
        // -----------------------------------
        if !NT::PV && !in_check {
            // Reverse futility pruning (static null move pruning).
            if can_apply_rfp(depth, improving, eval, beta) {
                return beta;
            }

            // Razoring.
            if can_apply_razoring(depth, eval, alpha, NT::PV) {
                let v = self.qsearch::<OffPV>(t, tt, alpha, beta);
                // If the qsearch still can't catch up, cut this node.
                if v <= alpha {
                    return v;
                }
            }

            // Null move pruning.
            if !singular && can_apply_nmp(&self.board, t, depth, improving, eval, beta) {
                let r = (nmp_base() + depth / nmp_factor()).min(depth);

                self.make_null(t);
                let value = -self.nwsearch(t, tt, child_pv, -beta + Eval(1), depth - r, !cutnode);
                self.undo_null(t);

                // cutoff above beta
                if value >= beta {
                    return beta;
                }
            }
        }

        // Internal Iterative reductions.
        if can_apply_iir(depth, NT::PV, cutnode, tt_move, tt_bound) {
            depth -= 1;
        }

        // -----------------------------------
        //              Probcut
        // -----------------------------------
        let pc_beta = beta + pc_beta_base() + (!improving as i32 * pc_beta_non_improving());

        if !NT::PV && beta.nonterminal() && depth >= 5 && !(tt_depth >= depth - 3 && tt_value < pc_beta) {
            let mut mp = MovePicker::new(SearchType::Pc, in_check, tt_move, pc_beta - t.ss().eval);
            let pc_depth = depth - 4;

            while let Some(m) = mp.next(&self.board, t) {
                // Ignore excluded move.
                if excluded == Some(m) {
                    continue;
                }

                self.make_move(m, t);

                // Do a quick qsearch to see if the move is worth looking at.
                let mut v = -self.qsearch::<OffPV>(t, tt, -pc_beta, -pc_beta + 1);

                // If it is, then do the full search.
                if v >= pc_beta {
                    v = -self.nwsearch(t, tt, pv, -pc_beta + 1, pc_depth, !cutnode)
                }

                self.undo_move(m, t);

                if v >= pc_beta {
                    tt.insert(self.board.state.hash, Bound::Lower, m, raw_value, v, pc_depth + 1, t.ply, tt_pv);

                    if v.nonterminal() {
                        return v;
                    }
                }
            }
        }

        // -----------------------------------
        //             Moves loop
        // -----------------------------------
        let mut best_move = Move::NONE;
        let mut best_value = -Eval::INFINITY;

        let mut caps_tried = MoveBuffer::default();
        let mut quiets_tried = MoveBuffer::default();

        let mut moves_tried = 0;
        let mut moves_exist = false;

        let lmp_margin = ((depth * depth + lmp_base()) / (2 - improving as i16)) as usize;
        let see_margins = [sp_noisy_margin() * (depth * depth) as i32, sp_quiet_margin() * depth as i32];

        let mut mp = MovePicker::new(SearchType::Pv, in_check, tt_move, Eval::DRAW);
        while let Some(m) = mp.next(&self.board, t) {
            debug_assert!(!m.is_none());
            moves_exist = true;

            // Ignore excluded move.
            if excluded == Some(m) {
                moves_tried += 1;
                continue;
            }

            let start_nodes = t.nodes;
            let is_quiet = m.flag().is_quiet();
            let hist_score = t.hist_score(&self.board, m);
            let mut new_depth = depth - 1;

            // -----------------------------------
            //          Move loop pruning
            // -----------------------------------
            if !NT::PV && !in_check && !mp.skip_quiets && best_value.nonterminal() {
                // History pruning.
                if can_apply_hp(depth, is_quiet, hist_score) {
                    mp.skip_quiets = true;
                }

                // Late move pruning.
                if can_apply_lmp(depth, moves_tried, lmp_margin) {
                    mp.skip_quiets = true;
                }

                // Futility pruning.
                if can_apply_fp(depth, eval, alpha, moves_tried) {
                    mp.skip_quiets = true;
                }
            }

            // SEE pruning.
            // If all captures happen on this move and we lose, prune this move.
            if depth <= sp_d_min()
                && best_value.nonterminal()
                && mp.stage > MPStage::PvNoisyWin
                && !self.board.see(m, Eval(see_margins[is_quiet as usize]))
            {
                moves_tried += 1;
                continue;
            };

            // -----------------------------------
            //             Extensions
            // -----------------------------------

            // Singular extensions: if the TT move is significantly better than all alternatives,
            // extend the search depth for this move as it's likely critical.
            if !NT::RT
                && depth >= ext_d_min()
                && !singular
                && m == tt_move
                && tt_value.nonterminal()
                && tt_bound.has(Bound::Lower)
                && tt_depth >= depth - 3
            {
                let ext_beta = (tt_value - depth * ext_mult()).max(-Eval::INFINITY);

                // Search all moves except the TT move at reduced depth.
                t.ss_mut().excluded = Some(m);
                let v = self.nwsearch(t, tt, child_pv, ext_beta, new_depth / 2, cutnode);
                t.ss_mut().excluded = None;

                // If no other move can reach the TT move's value, extend this move.
                let ext = if v < ext_beta {
                    1
                }
                // Negative extensions.
                else if tt_value >= beta {
                    -2 + NT::PV as Depth
                } else if cutnode {
                    -2
                }
                // Otherwise, don't extend.
                else {
                    0
                };

                new_depth += ext;
            }

            // -----------------------------------
            //             Make Move
            // -----------------------------------
            self.make_move(m, t);
            tt.prefetch(self.board.state.hash);

            let mut v = -Eval::INFINITY;

            // Late move reductions.
            // If we have already searched a lot of moves in this position, then we have probably
            // already looked at the best moves. We reduce the depth that we search the other moves
            // at accordingly.
            if can_apply_lmr(depth, moves_tried) {
                let mut r = lmr_base_reduction(depth, moves_tried);

                // Decrease reductions for good moves.
                r -= in_check as Depth;
                r -= self.board.in_check() as Depth;
                r -= (tt_depth >= depth) as Depth;
                r -= (hist_score / (if is_quiet { hist_quiet_div() } else { hist_noisy_div() })) as Depth;

                // Increase reductions for bad moves.
                r += !NT::PV as Depth;
                r += cutnode as Depth;
                r += !improving as Depth;
                r += tt_move.flag().is_noisy() as Depth;

                r = r.clamp(1, depth - 1);

                // Try reduced depth first.
                v = -self.nwsearch(t, tt, child_pv, -alpha, new_depth + 1 - r, true);

                // Re-search at full depth if the reduced search suggests the move is good.
                if v > alpha && r > 1 {
                    new_depth += (v > best_value + lmr_ver_e_min() + 2 * new_depth as i32) as Depth;
                    new_depth -= (v < best_value + new_depth) as Depth;
                    v = -self.nwsearch(t, tt, child_pv, -alpha, new_depth, !cutnode);
                }
            }
            // For moves that can't be reduced, or first move in non-PV, do null-window search
            else if !NT::PV || moves_tried > 0 {
                v = -self.nwsearch(t, tt, child_pv, -alpha, new_depth, !cutnode);
            };

            // For the first move in a PV node, or any move that beats alpha,
            // do a full-window search to get the exact score.
            if NT::PV && (moves_tried == 0 || v > alpha) {
                v = -self.pvsearch::<NT::Next>(t, tt, child_pv, -beta, -alpha, new_depth, false);
            }

            self.undo_move(m, t);

            if t.stop {
                return Eval::DRAW;
            }

            if NT::RT {
                t.tm.update_nodes(m, t.nodes - start_nodes);
            }

            // Update best move and alpha if we found a better move.
            if v > best_value {
                best_value = v;

                if v > alpha {
                    best_move = m;

                    if NT::PV {
                        pv.update(m, child_pv);
                    }

                    // Beta cutoff: this position is too good, opponent won't allow it.
                    if v >= beta {
                        break;
                    }

                    alpha = v;
                }
            }

            // Add move to history.
            if is_quiet {
                quiets_tried.push(m);
            } else if m.flag().is_cap() {
                caps_tried.push(m);
            }

            moves_tried += 1;
        }

        // No legal moves: checkmate or stalemate.
        if !moves_exist {
            return if in_check { Eval::mated_in(t.ply) } else { Eval::DRAW };
        }

        let bound = if best_value >= beta {
            // Insert this position in at a lower bound.
            // We stopped searching after the beta cutoff, as we proved the position is so strong
            // that the opponent will play to avoid it.
            // We don't know the exact value of the position, we just know it's at least beta.
            best_value = beta;

            // Update move ordering histories with a malus for moves that didn't cause beta cutoff, and a bonus for the move that did.
            t.update_history(best_move, depth, &self.board, &quiets_tried, &caps_tried);

            Bound::Lower
        } else if !NT::PV || best_move.is_none() {
            // Insert this position in at an upper bound.
            // If we never updated the best move, then none of the moves were better than alpha - so at best, the position is equal to alpha.
            best_value = alpha;
            Bound::Upper
        } else {
            // We have searched all the moves and have an exact bound for the score.
            // This means we can trust best_value, so insert at exact bound.
            Bound::Exact
        };

        // Update correction history.
        if !best_move.flag().is_cap() && !in_check && bound.is_usable(best_value, t.ss().eval) {
            t.update_corrhist(&self.board, best_value, depth);
        }

        // Store the result in the TT.
        if !singular {
            tt.insert(self.board.state.hash, bound, best_move, raw_value, best_value, depth, t.ply, tt_pv);
        }

        best_value
    }
}
