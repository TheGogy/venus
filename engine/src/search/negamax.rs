use chess::{
    MAX_DEPTH,
    movegen::ALL_MOVE,
    types::{eval::Eval, moves::Move},
};

use crate::{
    position::pos::Pos,
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
    tunables::params::tunables::*,
};

use super::{NodeType, pv::PVLine};

impl Pos {
    /// Negamax search function.
    /// This performs the majority of the searching, then drops into qsearch at the end.
    pub fn negamax<NT: NodeType>(
        &mut self,
        t: &mut Thread,
        tt: &TT,
        pv: &mut PVLine,
        mut alpha: Eval,
        mut beta: Eval,
        mut depth: usize,
    ) -> Eval {
        if t.should_stop() {
            t.stop = true;
            return Eval::DRAW;
        }

        let in_check = self.board.in_check();

        // Base case: depth = 0
        if depth == 0 && !in_check {
            // return self.evaluate();
            return self.qsearch::<NT::Next>(t, tt, pv, alpha, beta);
        }

        // Check extensions
        if in_check && depth < MAX_DEPTH {
            depth += 1;
        }

        pv.clear();

        t.seldepth = if NT::RT { 0 } else { t.seldepth.max(t.ply) };

        if !NT::RT {
            // Check for immediate draw.
            if self.board.is_draw(t.ply_from_null) {
                return Eval::dithered_draw(t.nodes as i32);
            }

            // Mate distance pruning. If we have already found a faster mate,
            // then we don't need to search this node.
            alpha = alpha.max(Eval::mated_in(t.ply));
            beta = beta.min(Eval::mate_in(t.ply + 1));

            if alpha >= beta {
                return alpha;
            }
        }

        let excluded = t.ss().excluded;
        let singular = !excluded.is_null();
        let mut se_possible = false;

        // -----------------------------------
        //             TT lookup
        // -----------------------------------
        let tt_entry = tt.probe(self.board.state.hash);
        let mut tt_move = Move::NULL;

        if let Some(tte) = tt_entry {
            let tt_depth = tte.depth();
            let tt_bound = tte.bound();
            let tt_value = tte.value(t.ply);

            tt_move = tte.mov();

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

            se_possible =
                !NT::RT && depth >= se_d_min() && !tt_value.is_tb_mate_score() && tt_bound != Bound::Upper && tt_depth >= depth - 3;
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
            if self.can_apply_rfp(t, depth, improving, eval, beta) {
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

        let child_pv = &mut PVLine::default();

        let old_alpha = alpha;

        let mut mp = match self.init_movepicker::<ALL_MOVE>(tt_move) {
            Some(mp) => mp,
            None => {
                return if in_check { Eval::mated_in(t.ply) } else { Eval::DRAW };
            }
        };

        while let Some((m, _)) = mp.next(&self.board, t) {
            assert!(m.is_valid());

            // Ignore excluded move.
            if m == excluded {
                continue;
            }

            let start_nodes = t.nodes;
            let is_quiet = m.flag().is_quiet();

            let mut new_depth = depth - 1;

            // Singular extensions.
            // If all moves look bad except one, extend that move.
            if se_possible && m == tt_move {
                let tt_value = tt_entry.unwrap().value(t.ply);
                let se_beta = (tt_value - Eval::from(depth * se_mult())).max(-Eval::INFINITY);

                t.ss_mut().excluded = m;
                let v = self.null_window_search(t, tt, pv, se_beta, new_depth / 2);
                t.ss_mut().excluded = Move::NULL;

                new_depth += (v < se_beta) as usize;
            }

            self.make_move(m, t);

            let v = -self.negamax::<NT::Next>(t, tt, child_pv, -beta, -alpha, new_depth);

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
                    t.update_tables(m, depth, &self.board, quiets_tried, caps_tried);
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

        self.store_search_result(t, tt, best_eval, alpha, beta, old_alpha, best_move, depth, NT::PV);

        alpha
    }
}
