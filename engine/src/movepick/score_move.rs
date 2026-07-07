use chess::{
    movegen::{Allmv, Noisy, Quiet},
    types::{
        bitboard::Bitboard,
        board::Board,
        eval::Eval,
        moves::{Move, MoveFlag},
        piece::Piece,
    },
};

use super::{MovePicker, SearchType};
use crate::{
    history::noisyhist::CAP_HIST_MAX,
    threading::thread::Thread,
    tunables::params::tunables::{mp_gc_bonus, mp_givecheck_see},
};

/// The value of the victim we are capturing with this move.
const MVV: [i32; Piece::NUM] = [0, 2400, 2400, 4800, 9600, 0];

fn capture_value(b: &Board, m: Move) -> i32 {
    debug_assert!(m.flag().is_cap());
    MVV[b.captured(m).pt().idx()]
}

impl MovePicker {
    /// Generate all quiet moves and score them.
    pub fn gen_score_quiets(&mut self, b: &Board, t: &Thread) {
        let prev_piecetos = t.get_prev_piecetos();

        let mut threat_masks = [Bitboard::EMPTY; Piece::NUM];

        let opp = !b.stm;

        // Pawns are never threatened

        // Knights and bishops are threatened by pawns.
        threat_masks[Piece::Knight.idx()] = b.all_pawn_atk(opp);
        threat_masks[Piece::Bishop.idx()] = b.all_pawn_atk(opp);

        // Rooks are threatened by pawns, knights, bishops.
        threat_masks[Piece::Rook.idx()] = threat_masks[Piece::Bishop.idx()] | b.all_knight_atk(opp) | b.all_bishop_atk(opp);

        // Queens are threatned by pawns, knights, bishops, rooks.
        threat_masks[Piece::Queen.idx()] = threat_masks[Piece::Rook.idx()] | b.all_rook_atk(opp);

        // If kings are under threat, we would be in evasions.

        b.enumerate_moves::<_, Quiet>(|m| {
            // We've already picked the TT move if it exists.
            if m == self.tt_move || m == self.killer {
                return;
            }

            let mut score = t.hist_quiet.get_bonus(b.stm, m);

            for (hist_cont, &pt_opt) in t.hist_conts.iter().zip(prev_piecetos.iter()) {
                if let Some(pt) = pt_opt {
                    score += hist_cont.get_bonus(m, pt);
                }
            }

            score += i32::from(b.gives_check_fast(m) && b.see(m, Eval(mp_givecheck_see()))) * mp_gc_bonus();

            let threat = threat_masks[b.pc_at(m.src()).pt().idx()];
            let v = i32::from(threat.has(m.src())) - i32::from(threat.has(m.dst()));
            score += v * MVV[b.pc_at(m.src()).pt().idx()] * 10;

            self.move_list.push_good(m, score);
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
            let score = match m.flag() {
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
            let threshold = if self.searchtype == SearchType::Pv { Eval(-score / 32) } else { self.see_threshold };
            if b.see(m, threshold) && !m.flag().is_underpromo() {
                self.move_list.push_good(m, score);
            } else {
                self.move_list.push_bad(m, score);
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
            let score = if m.flag().is_cap() {
                NOISY_BASE + capture_value(b, m)
            } else {
                let ch = t.pieceto_at(1).map_or(0, |pt| t.hist_conts[0].get_bonus(m, pt));
                t.hist_quiet.get_bonus(b.stm, m) + ch
            };

            self.move_list.push_good(m, score);
        });
    }
}
