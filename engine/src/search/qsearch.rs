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

use super::NodeType;

impl Pos {
    /// Quiescence search.
    /// We use this to avoid the "horizon" effect, by continuing the search
    /// until all captures have been made.
    pub fn qsearch<NT: NodeType>(&mut self, t: &mut Thread, tt: &TT, mut alpha: Eval, beta: Eval) -> Eval {
        // Check for upcoming repetition.
        if alpha < Eval::DRAW && self.board.upcoming_repetition(t.ply) {
            alpha = Eval::dithered_draw(t.nodes as i32);
            if alpha >= beta {
                return alpha;
            }
        }

        // Update seldepth.
        t.seldepth = t.seldepth.max(t.ply);

        let in_check = self.board.in_check();

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
        let mut tt_move = Move::NONE;
        let mut tt_eval = Eval::NONE;
        let mut tt_value = Eval::NONE;
        let mut tt_bound = Bound::None;
        let mut tt_pv = false;

        if let Some(tte) = tt.probe(self.board.state.hash) {
            tt_move = tte.mov();
            tt_eval = tte.eval();
            tt_value = tte.value(t.ply);
            tt_bound = tte.bound();
            tt_pv = tte.pv();
        };

        // Use the TT score instead if possible.
        if !NT::PV && tt_value.is_valid() && tt_bound.is_usable(tt_value, beta) {
            return tt_value;
        }

        // -----------------------------------
        //            Static Eval
        // -----------------------------------
        let mut best_eval = -Eval::INFINITY;
        let mut static_eval = Eval::NONE;
        let mut futility = Eval::NONE;

        // Don't evaluate positions in check.
        if in_check {
            t.ss_mut().eval = Eval::NONE;
        } else {
            // Get eval from tt if possible.
            static_eval = if tt_eval.is_valid() { tt_eval } else { self.evaluate(&mut t.nnue) };
            t.ss_mut().eval = static_eval;
            best_eval = static_eval;
            futility = best_eval + fp_qs_base();

            // If the tt bound is tighter, use it instead.
            if tt_value.is_valid() && tt_bound.is_usable(tt_value, best_eval) {
                best_eval = tt_value
            }

            // If the static eval is at least beta, return here.
            if best_eval >= beta {
                return (best_eval + beta) / 2;
            }

            // Update alpha.
            alpha = alpha.max(best_eval);
        };

        // -----------------------------------
        //             Moves loop
        // -----------------------------------
        let mut best_move = Move::NONE;
        let mut made_move = false;

        let mut mp = MovePicker::new(SearchType::Qs, in_check, tt_move);

        while let Some(m) = mp.next(&self.board, t) {
            made_move = true;

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
            let v = -self.qsearch::<NT::Next>(t, tt, -beta, -alpha);
            self.undo_move(m, t);

            // Update best.
            if v > best_eval {
                best_eval = v;

                if v > alpha {
                    best_move = m;

                    if v >= beta {
                        break;
                    }

                    alpha = best_eval;
                }
            }
        }

        // See if the position is checkmate.
        if in_check && !made_move {
            return Eval::mated_in(t.ply);
        }

        if best_eval >= beta && !best_eval.is_tb_mate() {
            best_eval = (best_eval + beta) / 2;
        }

        // Store result in tt.
        let result_bound = if best_eval >= beta { Bound::Lower } else { Bound::Upper };
        tt.insert(self.board.state.hash, result_bound, best_move, best_eval, static_eval, 0, t.ply, tt_pv);

        best_eval
    }
}
