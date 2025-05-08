use chess::types::{eval::Eval, moves::Move};

use crate::{
    position::pos::Pos,
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
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
}
