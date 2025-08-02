use chess::types::eval::Eval;

use crate::{
    position::Position,
    threading::{pv::PVLine, thread::Thread},
    tt::table::TT,
    tunables::params::tunables::*,
};

use super::Root;

impl Position {
    /// Iterative deepening loop.
    /// Search at increasing depth until we should stop.
    pub fn iterative_deepening<const MAIN: bool>(&mut self, t: &mut Thread, tt: &TT) {
        while t.should_start_iter() {
            let eval = self.asp_window(t, tt);

            // If search was stopped (time limit or manually), don't use the incomplete result.
            if t.stop {
                break;
            }

            t.eval = eval;
            t.depth += 1;

            if MAIN {
                println!(
                    "info depth {} seldepth {} score {} hashfull {} {} {}",
                    t.depth,
                    t.seldepth,
                    t.eval,
                    tt.hashfull(),
                    t.tm,
                    t.pv.to_uci(&self.board.castlingmask)
                );
            }
        }
    }

    /// Aspiration window. Keep searching until we find something within the window.
    fn asp_window(&mut self, t: &mut Thread, tt: &TT) -> Eval {
        let mut pv = PVLine::default();
        let mut alpha = -Eval::INFINITY;
        let mut beta = Eval::INFINITY;
        let mut delta = asp_window_default();

        let full_depth = t.depth + 1;
        let mut search_depth = t.depth + 1;

        // Setup aspiration window once we have a reliable evaluation from previous iterations.
        // At very shallow depths, the evaluation can be too unstable.
        if search_depth >= asp_window_d_min() {
            alpha = (t.eval - delta).max(-Eval::INFINITY);
            beta = (t.eval + delta).min(Eval::INFINITY);
        }

        loop {
            // Search within the window
            let v = self.pvsearch::<Root>(t, tt, &mut pv, alpha, beta, search_depth, false);

            if t.stop {
                return -Eval::INFINITY;
            }

            // Search failed low (fell below alpha).
            // This means the position is worse than we thought.
            // Move beta towards alpha to narrow the window from above, and
            // expand alpha downward to catch the actual value.
            if v <= alpha {
                beta = (alpha + beta) / 2;
                alpha = (v - delta).max(-Eval::INFINITY);
                search_depth = full_depth;
            }
            // Search failed high (exceeded beta).
            // This means the position is better than we thought.
            // Expand beta upward to catch the actual value, and save the PV.
            else if v >= beta {
                beta = (v + delta).min(Eval::INFINITY);
                t.pv = pv.clone();

                // Depth reduction on fail-high.
                // When we fail high, we often don't need full depth to prove the position is good.
                if search_depth > 1 && v.nonterminal() {
                    search_depth -= 1;
                }
            }
            // Found result within the window: return.
            else {
                t.pv = pv;
                return v;
            }

            // Gradually expand the aspiration window for the next attempt.
            // If we keep failing outside the window, it means the position's value
            // has changed significantly, so we need a wider search window.
            delta += delta / 2;
        }
    }
}
