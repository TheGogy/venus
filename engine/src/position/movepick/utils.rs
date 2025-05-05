use chess::types::moves::Move;

use super::MovePicker;

impl<const QUIET: bool> MovePicker<QUIET> {
    /// Yield the first move satisfying the given predicate.
    #[inline]
    pub fn find_pred(&mut self, start: usize, end: usize, pred: impl Fn(Move) -> bool) -> Option<Move> {
        if start >= end {
            return None;
        }

        for i in start..end {
            let m = self.moves.moves[i];

            if pred(m) {
                self.moves.moves.swap(i, start);
                return Some(m);
            }
        }

        None
    }

    /// Single iteration of insertion sort
    #[inline]
    pub fn partial_sort(&mut self, end: usize) -> Option<(Move, i32)> {
        if self.idx_cur == end {
            return None;
        }

        let mut best_score = self.scores[self.idx_cur];
        let mut best_index = self.idx_cur;
        for i in (self.idx_cur + 1)..end {
            if self.scores[i] > best_score {
                best_score = self.scores[i];
                best_index = i;
            }
        }

        self.moves.moves.swap(self.idx_cur, best_index);
        self.scores.swap(self.idx_cur, best_index);
        self.idx_cur += 1;

        Some((self.moves.moves[self.idx_cur - 1], best_score))
    }
}
