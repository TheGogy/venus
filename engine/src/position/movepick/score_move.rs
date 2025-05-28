use chess::{
    movegen::{MG_ALLMV, MG_NOISY, MG_QUIET},
    types::{board::Board, eval::Eval, moves::Move, piece::Piece},
};

use crate::{
    history::conthist::CONT_NUM, maybe_const, position::movepick::SearchType, threading::thread::Thread, tunables::params::tunables::*,
};

use super::MovePicker;

impl MovePicker {
    /// Generate all quiet moves and score them.
    pub fn gen_score_quiets(&mut self, b: &Board, t: &Thread) {
        const THREAT_Q: i32 = 32768;
        const THREAT_R: i32 = 16384;
        const THREAT_M: i32 = 16384;

        maybe_const!(CH_SCALE: [i32; CONT_NUM] = [ch_scale_0(), ch_scale_1(), ch_scale_2(), ch_scale_3(), ch_scale_4(), ch_scale_5()]);

        let opp = !b.stm;
        let threat_pawns = b.atk_from(Piece::Pawn, opp);
        let threat_minor = b.atk_from(Piece::Knight, opp) | b.atk_from(Piece::Bishop, opp) | threat_pawns;
        let threat_major = b.atk_from(Piece::Rook, opp) | threat_minor;

        let prev_piecetos = t.get_prev_moves();

        b.enumerate_moves::<_, MG_QUIET>(|m| {
            // We've already picked the TT move if it exists.
            if m == self.tt_move {
                return;
            }

            let (src, dst) = (m.src(), m.dst());
            let pt = b.pc_at(src).pt();

            // Get threat score.
            #[rustfmt::skip]
            let mut s = match pt {
                Piece::Queen                  => (threat_major.get_bit(src) as i32 - threat_major.get_bit(dst) as i32) * THREAT_Q,
                Piece::Rook                   => (threat_minor.get_bit(src) as i32 - threat_minor.get_bit(dst) as i32) * THREAT_R,
                Piece::Bishop | Piece::Knight => (threat_pawns.get_bit(src) as i32 - threat_pawns.get_bit(dst) as i32) * THREAT_M,
                _ => 0,
            };

            s += t.hist_quiet.get_bonus(b.stm, m);

            for i in 0..CONT_NUM {
                if let Some(pt) = prev_piecetos[i] {
                    s += (t.hist_conts[i].get_bonus(m, pt) * CH_SCALE[i]) / 1000;
                }
            }

            self.ml_quiet.add(m, s);
        });
    }

    /// Generate all noisy moves and score them.
    pub fn gen_score_noisies(&mut self, b: &Board, t: &Thread) {
        b.enumerate_moves::<_, MG_NOISY>(|m| {
            // We've already picked the TT move if it exists.
            if m == self.tt_move {
                return;
            }

            let mut s = capture_value(b, m) * 16;
            s += t.hist_noisy.get_bonus(b, m);

            if m.flag().is_promo() {
                s += 16384;
            }

            // See if we should put this move into the good noisy moves or bad noisy moves.
            let see_threshold = if self.searchtype == SearchType::Pv { Eval::from_raw(-s / 32) } else { self.see_threshold };
            if b.see(m, see_threshold) && !m.flag().is_underpromo() {
                self.ml_noisy_win.add(m, s);
            } else {
                self.ml_noisy_loss.add(m, s);
            }
        });
    }

    /// Generate all evasion moves and score them.
    pub fn gen_score_evasions(&mut self, b: &Board, t: &Thread) {
        const NOISY_BASE: i32 = 1_000_000;
        b.enumerate_moves::<_, MG_ALLMV>(|m| {
            let s = if m.flag().is_noisy() {
                NOISY_BASE + capture_value(b, m)
            } else {
                let ch = t.pieceto_at(1).map(|pt| t.hist_conts[0].get_bonus(m, pt)).unwrap_or(1);
                t.hist_quiet.get_bonus(b.stm, m) + ch
            };

            self.ml_quiet.add(m, s);
        });
    }
}

fn capture_value(b: &Board, m: Move) -> i32 {
    // We need an extra zero here because not all noisy moves are captures:
    // a queen promotion counts as a noisy move, even if it is not a capture.
    // As such, the captured piece is empty.
    maybe_const!(P_VAL: [i32; Piece::NUM + 1] = [val_pawn(), val_knight(), val_bishop(), val_rook(), val_queen(), 0, 0]);
    P_VAL[b.captured(m).pt().idx()]
}
