use chess::{MAX_MOVES, types::moves::Move};

use super::MovePicker;

impl MovePicker {
    // Selects the next best move, and moves it to the front of the list.
    // We then move the current index towards the middle of the list.
    pub fn select_upto<const IS_LEFT: bool>(&mut self, end: usize) -> Move {
        let cur = self.cur;

        let mut best_idx = cur;
        let mut best_scr = self.scs[cur];

        let mut i = cur;
        while i != end {
            assert!(i < MAX_MOVES);
            assert!((IS_LEFT && i < end) || (!IS_LEFT && i > end));
            assert!(self.mvs[i].is_valid());

            let s = self.scs[i];

            if s > best_scr {
                best_idx = i;
                best_scr = s;
            }

            if IS_LEFT { i += 1 } else { i -= 1 }
        }

        self.swap(cur, best_idx);

        if IS_LEFT {
            self.cur += 1
        } else {
            self.cur -= 1
        }

        self.mvs[cur]
    }

    /// Swaps moves and scores.
    pub const fn swap(&mut self, a: usize, b: usize) {
        self.mvs.swap(a, b);
        self.scs.swap(a, b);
    }

    /// Adds in a move and score at a given index.
    pub const fn insert(&mut self, m: Move, s: i32, i: usize) {
        self.mvs[i] = m;
        self.scs[i] = s;
    }
}
