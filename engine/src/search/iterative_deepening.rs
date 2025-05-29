use chess::types::eval::Eval;

use crate::{position::pos::Pos, threading::thread::Thread, tt::table::TT, tunables::params::tunables::*};

use super::{Root, pv::PVLine};

impl Pos {
    /// Iterative deepening loop.
    /// Search at increasing depth until we should stop.
    pub fn iterative_deepening<const MAIN: bool>(&mut self, t: &mut Thread, tt: &TT) {
        while t.should_start_iter() {
            let eval = self.asp_window(t, tt);

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
                    t.clock,
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

        // Setup aspiration window once we are over the min depth.
        if search_depth >= asp_window_d_min() {
            alpha = (t.eval - delta).max(-Eval::INFINITY);
            beta = (t.eval + delta).min(Eval::INFINITY);
        }

        loop {
            // Search within the window
            let v = self.pvs::<Root>(t, tt, &mut pv, alpha, beta, search_depth, false);

            if t.stop {
                return -Eval::INFINITY;
            }

            // Search failed low: reset depth and close window.
            if v <= alpha {
                alpha = (alpha - delta).max(-Eval::INFINITY);
                beta = (alpha + beta) / 2;
                search_depth = full_depth;
            }
            // Search failed high: reduce depth, open window.
            else if v >= beta {
                beta = (beta + delta).min(Eval::INFINITY);
                t.pv = pv.clone();

                if search_depth > 1 && v.abs() < Eval::LONGEST_TB_MATE {
                    search_depth -= 1;
                }
            }
            // Found result within the window: return.
            else {
                t.pv = pv;
                return v;
            }

            // Expand search window.
            delta += delta / 2;
        }
    }
}
