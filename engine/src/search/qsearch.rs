use chess::{
    MAX_DEPTH,
    movegen::TAC_ONLY,
    types::{eval::Eval, moves::Move},
};

use crate::{
    position::pos::Pos,
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
};

use super::{NodeType, pv::PVLine};

impl Pos {
    /// Quiescence search.
    /// We use this to avoid the "horizon" effect, by continuing the search
    /// until all captures have been made.
    pub fn qsearch<NT: NodeType>(&mut self, t: &mut Thread, tt: &TT, pv: &mut PVLine, mut alpha: Eval, beta: Eval) -> Eval {
        // Update seldepth.
        t.seldepth = t.seldepth.max(t.ply);

        // Clear PV
        pv.clear();

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
        let mut tt_move = Move::NULL;

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

        let eval = self.static_eval(t, tt_entry, in_check);

        // If the static eval is at least beta, return here.
        if eval >= beta {
            return (eval + beta) / 2;
        }

        alpha = alpha.max(eval);

        // -----------------------------------
        //             Moves loop
        // -----------------------------------

        let mut best_eval = eval;
        let mut best_move = Move::NULL;

        let child_pv = &mut PVLine::default();

        // If we have moves in the position, process them.
        if let Some(mut mp) = self.init_movepicker::<TAC_ONLY>(tt_move) {
            while let Some((m, _)) = mp.next(&self.board, t) {
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
                            alpha = beta;
                            break;
                        }
                    }
                }
            }
        }
        // No moves left.
        // If we are in check, that's checkmate.
        else if in_check {
            return Eval::mated_in(t.ply);
        }

        if best_eval > beta && !best_eval.abs().is_tb_mate_score() {
            best_eval = (best_eval + beta) / 2;
        }

        self.store_search_result(t, tt, best_eval, alpha, beta, old_alpha, best_move, 0, t.ss().ttpv);

        best_eval
    }
}
