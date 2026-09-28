use chess::{
    movegen::{Allmv, Noisy, Quiet},
    types::{bitboard::Bitboard, board::Board, eval::Eval, piece::Piece},
};

use super::MovePicker;
use crate::{
    position::see::see,
    threading::thread::Thread,
    tunables::params::{
        piece_value,
        tunables::{mp_gc_bonus, mp_givecheck_see, mp_noisy_captured_pc_value, mp_quiet_threat_pc_value},
    },
};

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
            if m == self.tt_move {
                return;
            }

            let (src, dst) = (m.src(), m.dst());
            let pc = b.pc_at(src);

            let mut score = t.hist_quiet.get_bonus(b.stm, m);

            for (hist_cont, &pt_opt) in t.hist_conts.iter().zip(prev_piecetos.iter()) {
                if let Some(pt) = pt_opt {
                    score += hist_cont.get_bonus(m, pt);
                }
            }

            score += i32::from(b.gives_check_fast(m) && see(b, m, Eval(mp_givecheck_see()))) * mp_gc_bonus();

            let threat = threat_masks[pc.pt().idx()];
            let v = i32::from(threat.has(src)) - i32::from(threat.has(dst));
            score += v * piece_value(pc) * mp_quiet_threat_pc_value();

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

            let score = piece_value(b.captured(m)) * mp_noisy_captured_pc_value() + t.hist_noisy.get_bonus(b, m);
            self.move_list.push_good(m, score);
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
                NOISY_BASE + piece_value(b.captured(m))
            } else {
                let ch = t.pieceto_at(1).map_or(0, |pt| t.hist_conts[0].get_bonus(m, pt));
                t.hist_quiet.get_bonus(b.stm, m) + ch
            };

            self.move_list.push_good(m, score);
        });
    }
}
