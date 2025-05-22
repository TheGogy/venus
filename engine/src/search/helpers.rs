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
    pub fn null_window_search(&mut self, t: &mut Thread, tt: &TT, pv: &mut PVLine, value: Eval, depth: i16, cutnode: bool) -> Eval {
        self.negamax::<OffPV>(t, tt, pv, value - 1, value, depth, cutnode)
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
        depth: i16,
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

            t.ss_mut().eval = if tt_eval == -Eval::INFINITY { self.evaluate(&mut t.nnue) } else { tt_eval };

            // If the tt eval is a tighter bound than the static eval, use it instead.
            // Otherwise, just use static eval
            tte.get_tightest(t.ss().eval, t.ply)

        // If nothing else, evaluate the position from scratch.
        } else {
            t.ss_mut().eval = self.evaluate(&mut t.nnue);
            t.ss().eval
        }
    }
}

/// Reverse futility pruning.
/// If the eval is well above beta, then we assume it will hold above beta.
pub fn can_apply_rfp(t: &Thread, depth: i16, improving: bool, eval: Eval, beta: Eval) -> bool {
    let rfp_margin = rfp_mult() * Eval::from_raw(depth as i32) - rfp_improving_margin() * Eval::from_raw(improving as i32);
    !t.ss().ttpv && depth <= rfp_d_min() && eval - rfp_margin >= beta
}

/// Late move reductions.
/// Reduce the search depth for moves with bad move ordering.
pub fn can_apply_lmr(depth: i16, moves_tried: usize, is_pv: bool) -> bool {
    depth >= lmr_d_min() && moves_tried as i16 >= lmr_m_min() + lmr_root_bonus() * is_pv as i16
}

/// Get the late move reduction amount.
pub fn lmr_reduction(depth: i16, moves_tried: usize) -> i16 {
    #[cfg(not(feature = "tune"))]
    {
        static LMR_TABLE: [[i16; 64]; 64] = unsafe { std::mem::transmute(*include_bytes!(concat!(env!("OUT_DIR"), "/lmr.bin"))) };

        LMR_TABLE[depth.min(63) as usize][moves_tried.min(63) as usize]
    }

    #[cfg(feature = "tune")]
    {
        if depth == 0 || moves_tried == 0 {
            return 0;
        }

        let lmr_base = lmr_base_scaled() as f32 / 1000.0;
        let lmr_mult = lmr_mult_scaled() as f32 / 1000.0;

        (lmr_base + (depth as f32).ln() * (moves_tried as f32).ln() / lmr_mult) as i16
    }
}
