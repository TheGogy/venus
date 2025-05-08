use chess::types::{
    board::Board,
    eval::Eval,
    moves::{Move, MoveFlag},
    piece::Piece,
};

use crate::{history::noisyhist::NOISY_MAX, threading::thread::Thread};

use super::MovePicker;
pub const TAC_GOOD: i32 = 2_000_000;
pub const TAC_BAD: i32 = 1_000_000;

/// Scoring methods for quiet moves.
impl<const QUIET: bool> MovePicker<QUIET> {
    pub fn score_quiets(&mut self, board: &Board, thread: &Thread) {
        thread.assign_history_scores(
            board.stm,
            &self.moves.moves[self.idx_cur..self.idx_noisy_bad],
            &mut self.scores[self.idx_cur..self.idx_noisy_bad],
        );
    }
}

/// Scoring methods for noisy moves.
impl<const QUIET: bool> MovePicker<QUIET> {
    #[rustfmt::skip]
    fn score_single(&self, m: Move, board: &Board, thread: &Thread) -> i32 {
        const MVV: [i32; Piece::NUM] = [0, 2400, 2400, 4800, 9600, 0];
        const PROMO: i32 = NOISY_MAX + MVV[Piece::Queen.index()] + 1;

        // Enpassant / capture promotions (to queen) are always good
        let score = match m.flag() {
            MoveFlag::CPromoQ      => return TAC_GOOD + PROMO,
            MoveFlag::PromoQ       => PROMO,
            t if t.is_underpromo() => return TAC_BAD,
            _                      => MVV[board.captured(m).pt().index()] + thread.hist_noisy.get_bonus(board, m),
        };

        if board.see(m, Eval::DRAW) { score + TAC_GOOD } else { score + TAC_BAD }
    }

    pub fn score_tacticals(&mut self, board: &Board, thread: &Thread) {
        let mut i = self.idx_cur;
        self.idx_quiets = self.idx_cur;

        while i < self.idx_noisy_bad {
            let m = self.moves.moves[i];

            if !QUIET || !m.flag().is_quiet() {
                let score = self.score_single(m, board, thread);

                if score >= TAC_GOOD {
                    self.moves.moves.swap(i, self.idx_quiets);
                    self.scores[self.idx_quiets] = score;
                    self.idx_quiets += 1;
                    i += 1;
                } else {
                    self.idx_noisy_bad -= 1;
                    self.moves.moves.swap(i, self.idx_noisy_bad);
                    self.scores[self.idx_noisy_bad] = score;
                }
            } else {
                i += 1;
            }
        }
    }
}
