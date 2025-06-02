use chess::{MAX_MOVES, types::moves::Move};

use super::MovePickerNew;

impl MovePickerNew {
    /// Selects the best score up to the given index, and increments current index.
    pub fn select_upto<const FORWARD: bool>(&mut self, end: usize) -> Move {
        let cur = self.cur;

        let mut best_idx = cur;
        let mut best_scr = self.scs[cur];

        let mut i = cur;
        while i != end {
            assert!(i < MAX_MOVES);
            assert!(self.mvs[i].is_valid());

            let s = self.scs[i];

            if s > best_scr {
                best_idx = i;
                best_scr = s;
            }

            if FORWARD { i += 1 } else { i -= 1 }
        }

        self.swap(cur, best_idx);

        if FORWARD {
            self.cur += 1
        } else {
            self.cur -= 1
        }

        self.mvs[cur]
    }

    /// Swaps moves and scores.
    pub fn swap(&mut self, a: usize, b: usize) {
        self.mvs.swap(a, b);
        self.scs.swap(a, b);
    }
}
