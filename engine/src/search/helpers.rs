use chess::types::{eval::Eval, moves::Move};

use crate::{
    position::pos::Pos,
    threading::thread::Thread,
    tt::{
        entry::{Bound, TTEntry},
        table::TT,
    },
    tunables::params::tunables::*,
};

use super::{OffPV, pv::PVLine};

impl Pos {
    /// Null window search.
    pub fn null_window_search(&mut self, t: &mut Thread, tt: &TT, pv: &mut PVLine, value: Eval, depth: usize) -> Eval {
        self.negamax::<OffPV>(t, tt, pv, value - 1, value, depth)
    }

    /// Stores the search result into the TT.
    #[allow(clippy::too_many_arguments)]
    pub fn store_search_result(
        &self,
        t: &mut Thread,
        tt: &TT,
        best_eval: Eval,
        alpha: Eval,
        beta: Eval,
        old_alpha: Eval,
        best_move: Move,
        depth: usize,
        pv: bool,
    ) {
        let bound = if best_eval >= beta {
            Bound::Lower
        } else if best_eval > old_alpha {
            Bound::Exact
        } else {
            Bound::Upper
        };

        tt.insert(self.board.state.hash, bound, best_move, t.ss().eval, alpha, depth, t.ply, pv);
    }

    /// Static evaluation of the position.
    /// We try to get the static eval from previous sources if possible, otherwise evaluate from
    /// scratch.
    pub fn static_eval(&self, t: &mut Thread, tt_entry: Option<TTEntry>, in_check: bool) -> Eval {
        // Don't evaluate positions in check.
        if in_check {
            t.ss_mut().eval = -Eval::INFINITY;
            -Eval::INFINITY

        // If we have already evaluated this position use that instead.
        } else if let Some(tte) = tt_entry {
            let tt_eval = tte.eval();

            t.ss_mut().eval = if tt_eval == -Eval::INFINITY { self.evaluate() } else { tt_eval };

            // If the tt eval is a tighter bound than the static eval, use it instead.
            // Otherwise, just use static eval
            tte.get_tightest(t.ss().eval, t.ply)

        // If nothing else, evaluate the position from scratch.
        } else {
            t.ss_mut().eval = self.evaluate();
            t.ss().eval
        }
    }
}

/// Pruning
impl Pos {
    /// Reverse futility pruning.
    /// If the eval is well above beta, then we assume it will hold above beta.
    pub fn can_apply_rfp(&self, t: &Thread, depth: usize, improving: bool, eval: Eval, beta: Eval) -> bool {
        let rfp_margin = rfp_mult() * Eval::from_raw(depth as i32) - rfp_improving_margin() * Eval::from_raw(improving as i32);
        !t.ss().ttpv && depth <= rfp_d_min() && eval - rfp_margin >= beta
    }
}
