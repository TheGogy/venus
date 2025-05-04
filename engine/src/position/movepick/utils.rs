use chess::types::moves::Move;

use super::MovePicker;

impl<const QUIET: bool> MovePicker<QUIET> {
    /// Sort between current index and end.
    pub fn partial_sort(&mut self, end: usize) -> Option<Move> {
        let cur = self.idx_cur;

        if cur == end {
            return None;
        }

        let mut best_idx = cur;
        let mut best_scr = self.scores[cur];

        for i in (cur + 1)..end {
            if self.scores[i] > best_scr {
                best_idx = i;
                best_scr = self.scores[i];
            }
        }

        self.moves.swap(cur, best_idx);
        self.scores.swap(cur, best_idx);

        self.idx_cur += 1;
        Some(self.moves.at(cur))
    }
}
