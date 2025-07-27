use chess::{
    movegen::{Allmv, Noisy, Quiet},
    types::{
        board::Board,
        eval::Eval,
        moves::{Move, MoveFlag},
        piece::Piece,
    },
};

use crate::{history::capturehist::CAP_HIST_MAX, movepick::move_list::LEFT, threading::thread::Thread};

use super::{MovePicker, SearchType, move_list::RIGHT};

/// The value of the victim we are capturing with this move..
const MVV: [i32; Piece::NUM] = [0, 2400, 2400, 4800, 9600, 0];

fn capture_value(b: &Board, m: Move) -> i32 {
    debug_assert!(m.flag().is_cap());
    MVV[b.captured(m).pt().idx()]
}

impl MovePicker {
    /// Generate all quiet moves and score them.
    pub fn gen_score_quiets(&mut self, b: &Board, t: &Thread) {
        let prev_piecetos = t.get_prev_piecetos();

        b.enumerate_moves::<_, Quiet>(|m| {
            // We've already picked the TT move if it exists.
            if m == self.tt_move {
                return;
            }

            let mut s = t.hist_quiet.get_bonus(b.stm, m);

            for (hist_cont, &pt_opt) in t.hist_conts.iter().zip(prev_piecetos.iter()) {
                if let Some(pt) = pt_opt {
                    s += hist_cont.get_bonus(m, pt);
                }
            }

            // Add to the front of the list.
            self.move_list.insert::<LEFT>(m, s);
        });
    }

    /// Generate all noisy moves and score them.
    pub fn gen_score_noisies(&mut self, b: &Board, t: &Thread) {
        b.enumerate_moves::<_, Noisy>(|m| {
            // We've already picked the TT move if it exists.
            if m == self.tt_move {
                return;
            }

            #[rustfmt::skip]
            let s = match m.flag() {
                // Regular queen promotions give us a queen for a pawn: best MVV trade.
                MoveFlag::PromoQ  => CAP_HIST_MAX + MVV[Piece::Queen.idx()] + 1,
                MoveFlag::CPromoQ => CAP_HIST_MAX + MVV[Piece::Queen.idx()] + capture_value(b, m),

                // Underpromotions are usually bad - we should probably promote to a queen.
                // (though these are captures).
                f if f.is_underpromo() => 0,

                // All other moves are captures, so this is safe.
                _ => capture_value(b, m) + t.hist_noisy.get_bonus(b, m)
            };

            // If this move doesn't pass the SEE test (or is an underpromotion),
            // move it back to the start with the other noisy moves.
            let threshold = if self.searchtype == SearchType::Pv { Eval(-s / 32) } else { self.see_threshold };
            if b.see(m, threshold) && !m.flag().is_underpromo() {
                // Good noisy move.
                self.move_list.insert::<LEFT>(m, s);
            } else {
                // Bad noisy move.
                self.move_list.insert::<RIGHT>(m, s);
            }
        });
    }

    /// Generate all evasion moves and score them.
    pub fn gen_score_evasions(&mut self, b: &Board, t: &Thread) {
        const NOISY_BASE: i32 = 1_000_000;

        b.enumerate_moves::<_, Allmv>(|m| {
            // We've already picked the TT move if it exists.
            if m == self.tt_move {
                return;
            }

            // Noisy moves should be pushed to the front of evasions.
            let s = if m.flag().is_cap() {
                NOISY_BASE + capture_value(b, m)
            } else {
                let ch = t.pieceto_at(1).map(|pt| t.hist_conts[0].get_bonus(m, pt)).unwrap_or(0);
                t.hist_quiet.get_bonus(b.stm, m) + ch
            };

            self.move_list.insert::<LEFT>(m, s);
        });
    }
}
