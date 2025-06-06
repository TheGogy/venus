use chess::types::{board::Board, eval::Eval, moves::Move};

use crate::{
    position::pos::Pos,
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
    tunables::params::tunables::*,
};

use super::{OffPV, pv::PVLine};

impl Pos {
    /// Null window search.
    pub fn nwsearch(&mut self, t: &mut Thread, tt: &TT, pv: &mut PVLine, value: Eval, depth: i16, cutnode: bool) -> Eval {
        self.pvsearch::<OffPV>(t, tt, pv, value - 1, value, depth, cutnode)
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
}

/// Reverse futility pruning.
/// If the eval is well above beta, then we assume it will hold above beta.
pub fn can_apply_rfp(t: &Thread, depth: i16, improving: bool, eval: Eval, beta: Eval) -> bool {
    let rfp_margin = rfp_mult() * Eval(depth as i32) - rfp_improving_margin() * Eval(improving as i32);
    !t.ss().ttpv && depth <= rfp_d_min() && eval - rfp_margin >= beta && !eval.is_tb_mate()
}

/// Null move pruning.
/// If the opponent gets a free move and we're still above beta, then our
/// position is probably so good we can just return beta.
pub fn can_apply_nmp(b: &Board, t: &Thread, depth: i16, improving: bool, eval: Eval, beta: Eval) -> bool {
    depth > nmp_d_min()
        && t.ply_from_null > 0
        && eval >= t.ss().eval
        && eval + nmp_improving_margin() * Eval(improving as i32) >= beta
        && !b.only_king_pawns_left()
}

// Internal iterative reductions.
// For PV nodes without a tt hit, decrease the depth.
pub fn can_apply_iir(depth: i16, tt_move: Move, is_pv: bool, cutnode: bool) -> bool {
    let iir_d_min = if is_pv { iir_pv_d_min() } else { iir_opv_d_min() };
    (is_pv || cutnode) && depth >= iir_d_min && tt_move.is_none()
}

/// Late move reductions.
/// Reduce the search depth for moves with bad move ordering.
pub fn can_apply_lmr(depth: i16, moves_tried: usize, is_pv: bool) -> bool {
    depth >= 2 && moves_tried as i16 > 1 + is_pv as i16
}

/// Get the late move reduction amount.
pub fn lmr_base_reduction(depth: i16, moves_tried: usize) -> i16 {
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

        let lmr_base = lmr_base() as f32 / 1024.0;
        let lmr_mult = lmr_mult() as f32 / 1024.0;

        (lmr_base + (depth as f32).ln() * (moves_tried as f32).ln() / lmr_mult) as i16
    }
}
