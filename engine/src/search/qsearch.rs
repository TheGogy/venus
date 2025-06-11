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

use super::{NodeType, pv::PVLine};

impl Pos {
    /// Quiescence search.
    /// We use this to avoid the "horizon" effect, by continuing the search
    /// until all captures have been made.
    pub fn qsearch<NT: NodeType>(&mut self, t: &mut Thread, tt: &TT, pv: &mut PVLine, mut alpha: Eval, beta: Eval) -> Eval {
        // Check for upcoming repetition.
        if alpha < Eval::DRAW && self.board.upcoming_repetition(t.ply) {
            alpha = Eval::dithered_draw(t.nodes as i32);
            if alpha >= beta {
                return alpha;
            }
        }

        // Update seldepth.
        t.seldepth = t.seldepth.max(t.ply);

        // Clear PV
        if NT::PV {
            pv.clear();
        }

        let in_check = self.board.in_check();
        let old_alpha = alpha;

        // Check if we are reaching max depth.
        if t.ply >= MAX_DEPTH {
            return if in_check { Eval::DRAW } else { self.evaluate(&mut t.nnue) };
        }

        // Stop searching if position is drawn.
        if self.board.is_draw(t.ply_from_null) {
            return Eval::DRAW;
        }

        // -----------------------------------
        //             TT lookup
        // -----------------------------------

        let tt_entry = tt.probe(self.board.state.hash);
        let mut tt_move = Move::NONE;

        if let Some(entry) = tt_entry {
            let tt_value = entry.value(t.ply);

            match entry.bound() {
                Bound::Exact => return tt_value,
                Bound::Lower if !NT::PV && !in_check && tt_value >= beta => return beta,
                Bound::Upper if !NT::PV && !in_check && tt_value <= alpha => return alpha,
                _ => tt_move = entry.mov(),
            }
        };

        t.ss_mut().ttpv = NT::PV;

        // -----------------------------------
        //            Static Eval
        // -----------------------------------
        let mut best_eval = if in_check {
            t.ss_mut().eval = -Eval::INFINITY;
            -Eval::INFINITY

        // If we have already evaluated this position use that instead.
        } else if let Some(tte) = tt_entry {
            let tt_eval = tte.eval();

            t.ss_mut().eval = if tt_eval == -Eval::INFINITY { self.evaluate(&mut t.nnue) } else { tt_eval };

            // If the tt eval is a tighter bound than the static eval, use it instead.
            // Otherwise, just use static eval.
            tte.get_tightest(t.ss().eval, t.ply)

        // If nothing else, evaluate the position from scratch.
        } else {
            // If we used a null move search last iteration, take the eval from that.
            t.ss_mut().eval = if t.ss_at(1).mov.is_null() { -t.ss_at(1).eval } else { self.evaluate(&mut t.nnue) };
            t.ss().eval
        };

        // If the static eval is at least beta, return here.
        if best_eval >= beta {
            return (best_eval + beta) / 2;
        }

        alpha = alpha.max(best_eval);

        // -----------------------------------
        //             Moves loop
        // -----------------------------------
        let mut best_move = Move::NONE;
        let mut moves_tried = 0;

        let child_pv = &mut PVLine::default();

        let mut mp = MovePicker::new(SearchType::Qs, in_check, tt_move);

        let futility = t.ss().eval + fp_qs_base();

        while let Some(m) = mp.next(&self.board, t) {
            moves_tried += 1;

            // -----------------------------------
            //              Pruning
            // -----------------------------------
            if !best_eval.is_loss() {
                if !self.board.in_check() && futility <= alpha && !self.board.see(m, Eval(1)) {
                    best_eval = best_eval.max(futility);
                    continue;
                }

                if !self.board.see(m, Eval(sp_qs_margin())) {
                    continue;
                }
            }

            self.make_move(m, t);
            let v = -self.qsearch::<NT::Next>(t, tt, child_pv, -beta, -alpha);
            self.undo_move(m, t);

            // Update best
            if v > best_eval {
                best_eval = v;

                if v > alpha {
                    best_move = m;

                    if NT::PV {
                        pv.clear();
                        pv.update(m, child_pv);
                    }

                    if v < beta {
                        alpha = v;
                    } else {
                        break;
                    }
                }
            }
        }

        // See if the position is checkmate.
        if in_check && moves_tried == 0 {
            return Eval::mated_in(t.ply);
        }

        if best_eval > beta && !best_eval.is_tb_mate() {
            best_eval = (best_eval + beta) / 2;
        }

        self.store_search_result(t, tt, best_eval, alpha, beta, old_alpha, best_move, 0, t.ss().ttpv);

        best_eval
    }
}
