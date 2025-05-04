use chess::types::{
    board::Board,
    eval::Eval,
    move_list::MoveList,
    moves::{Move, MoveFlag},
    piece::Piece,
};

use crate::history::{History, noisyhist::NOISY_MAX};

use super::MovePicker;

/// Scoring methods for quiet moves.
impl<const QUIET: bool> MovePicker<QUIET> {
    /// Score all quiet moves.
    pub fn score_quiets(&mut self, b: &Board, h: &History) {
        assert!(self.idx_cur < MoveList::SIZE);
        for i in self.idx_cur..self.idx_bad_noisy {
            self.scores[i] = h.quiet.get_bonus(b, self.moves.at(i))
        }
    }
}

/// Scoring methods for noisy moves.
impl<const QUIET: bool> MovePicker<QUIET> {
    const GOOD: i32 = 2_000_000;
    const BAD: i32 = 1_000_000;

    /// Score all noisy moves.
    pub fn score_noisies(&mut self, b: &Board, h: &History) {
        let mut i = self.idx_cur;
        self.idx_good_quiet = self.idx_cur;

        while i < self.idx_bad_noisy {
            let m = self.moves.at(i);

            if !QUIET || m.flag().is_noisy() {
                let score = self.score_noisy(m, b, h);

                if score >= Self::GOOD {
                    self.moves.swap(i, self.idx_good_quiet);
                    self.scores[self.idx_good_quiet] = score;
                    self.idx_good_quiet += 1;
                    i += 1;
                } else {
                    self.idx_bad_noisy -= 1;
                    self.moves.swap(i, self.idx_bad_noisy);
                    self.scores[self.idx_bad_noisy] = score;
                }
            } else {
                i += 1
            }
        }
    }

    /// Score a single noisy move.
    #[inline]
    fn score_noisy(&self, m: Move, b: &Board, h: &History) -> i32 {
        const MVVLVA: [i32; Piece::NUM] = [0, 2400, 2400, 4800, 9600, 0];
        const PROMO: i32 = NOISY_MAX + MVVLVA[Piece::Queen.index()] + 1;

        let score = match m.flag() {
            MoveFlag::CPromoQ => return Self::GOOD + PROMO,
            MoveFlag::PromoQ => PROMO,
            t if t.is_underpromo() => return Self::BAD,
            _ => MVVLVA[b.captured(m).pt().index()] + h.noisy.get_bonus(b, m),
        };

        if b.see(m, Eval::DRAW) { score + Self::GOOD } else { score + Self::BAD }
    }
}
