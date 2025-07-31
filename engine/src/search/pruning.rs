use chess::types::{Depth, board::Board, eval::Eval, moves::Move};

use crate::{threading::thread::Thread, tt::bits::Bound, tunables::params::tunables::*};

/// Reverse futility pruning.
// If our position is already so good that even without searching,
// we're likely to exceed beta, we can return beta immediately.
pub fn can_apply_rfp(depth: Depth, improving: bool, eval: Eval, beta: Eval) -> bool {
    let rfp_margin = rfp_mult() * Eval(depth as i32) - rfp_improving_margin() * Eval(improving as i32);
    depth <= rfp_d_max() && eval - rfp_margin >= beta
}

/// Razoring.
// If our static eval is far below alpha, do a quick qsearch to see
// if we can improve the position through tactics.
pub fn can_apply_razoring(depth: Depth, eval: Eval, alpha: Eval) -> bool {
    depth <= razoring_d_max() && eval.abs().0 <= razoring_e_max() && eval + (depth as i32 * razoring_d_mult()) < alpha
}

/// Null move pruning.
/// If the opponent gets a free move and we're still above beta, then our
/// position is probably so good we can just return beta.
pub fn can_apply_nmp(b: &Board, t: &Thread, depth: Depth, improving: bool, eval: Eval, beta: Eval) -> bool {
    depth > nmp_d_min()
        && t.ply_from_null > 0
        && eval >= t.ss().eval
        && eval + nmp_improving_margin() * Eval(improving as i32) >= beta
        && !b.only_king_pawns_left()
}

/// Internal iterative reductions.
// If we don't have a good move from the TT, reduce depth slightly
// to avoid spending too much time on potentially uninteresting positions.
pub fn can_apply_iir(depth: Depth, is_pv: bool, cutnode: bool, tt_move: Move, tt_bound: Bound) -> bool {
    (is_pv || cutnode) && tt_move.is_none() && tt_bound != Bound::Upper && depth >= iir_d_min() + 2 * cutnode as Depth
}

/// History Pruning.
/// If the current node has a bad history (and because of move sorting all subsequent moves will be
/// worse) then ignore quiet moves.
pub fn can_apply_hp(depth: Depth, is_quiet: bool, hist_score: i32) -> bool {
    is_quiet && depth <= hp_d_min() && hist_score < -hp_s_min()
}

/// Late move pruning.
/// If we have seen a lot of moves in this position already, and we don't expect something good
/// from this move, then we should skip the quiet moves.
pub fn can_apply_lmp(depth: Depth, moves_tried: usize, lmp_margin: usize) -> bool {
    depth <= lmp_d_min() && moves_tried >= lmp_margin
}

/// Futility pruning.
/// If our score is significantly below alpha, then this position is probably bad, then we should
/// skip the quiet moves.
pub fn can_apply_fp(depth: Depth, eval: Eval, alpha: Eval, moves_tried: usize) -> bool {
    let lmr_depth = depth - lmr_base_reduction(depth, moves_tried);
    let fp_margin = Eval(fp_base() + (lmr_depth as i32) * fp_mult());

    lmr_depth <= fp_d_min() && eval + fp_margin < alpha
}

/// Late move reductions.
/// Reduce the search depth for moves with bad move ordering.
pub fn can_apply_lmr(depth: Depth, moves_tried: usize, is_pv: bool) -> bool {
    depth >= 2 && moves_tried >= lmr_m_min() + is_pv as usize
}

/// Get the late move reduction amount.
pub fn lmr_base_reduction(depth: Depth, moves_tried: usize) -> Depth {
    #[cfg(not(feature = "tune"))]
    {
        static LMR_TABLE: [[Depth; 64]; 64] = unsafe { std::mem::transmute(*include_bytes!(concat!(env!("OUT_DIR"), "/lmr.bin"))) };

        LMR_TABLE[depth.min(63) as usize][moves_tried.min(63)]
    }

    #[cfg(feature = "tune")]
    {
        if depth == 0 || moves_tried == 0 {
            return 0;
        }

        let lmr_base = lmr_base() as f32 / 1024.0;
        let lmr_mult = lmr_mult() as f32 / 1024.0;

        (lmr_base + (depth as f32).ln() * (moves_tried as f32).ln() / lmr_mult) as Depth
    }
}
