use chess::{MAX_MOVES, types::moves::Move};

/// Scored movelist.
/// Keeps track of a movelist and its corresponding scores.
#[derive(Clone, Debug)]
pub struct ScoredMoveList {
    pub mvs: [Move; MAX_MOVES],
    pub scs: [i32; MAX_MOVES],
    pub idx: usize,
    pub end: usize,
}

impl Default for ScoredMoveList {
    fn default() -> Self {
        Self { mvs: [Move::NONE; MAX_MOVES], scs: [0; MAX_MOVES], idx: 0, end: 0 }
    }
}

impl ScoredMoveList {
    /// Whether we have moves in this scored move list.
    pub const fn non_empty(&self) -> bool {
        self.idx < self.end
    }

    /// Add a scored move to the move list.
    pub const fn add(&mut self, m: Move, s: i32) {
        self.mvs[self.end] = m;
        self.scs[self.end] = s;
        self.end += 1;
    }

    pub const fn swap(&mut self, a: usize, b: usize) {
        self.mvs.swap(a, b);
        self.scs.swap(a, b);
    }

    /// Get the next best move and score in the list, and increment the current index.
    pub fn next(&mut self) -> (Move, i32) {
        let cur = self.idx;

        let mut best_idx = cur;
        let mut best_scr = self.scs[cur];

        for i in self.idx..self.end {
            let s = self.scs[i];

            if s > best_scr {
                best_idx = i;
                best_scr = s;
            }
        }

        self.swap(cur, best_idx);
        self.idx += 1;

        (self.mvs[cur], self.scs[cur])
    }
}
