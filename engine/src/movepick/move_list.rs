use chess::{defs::MAX_MOVES, types::moves::Move};

#[derive(Clone, Debug)]
pub struct MoveList {
    moves: [(Move, i32); MAX_MOVES],
    good_cur: usize,
    good_end: usize,
    bad_cur: usize,
    bad_start: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self { moves: [(Move::NONE, 0); MAX_MOVES], good_cur: 0, good_end: 0, bad_cur: MAX_MOVES - 1, bad_start: MAX_MOVES - 1 }
    }
}

impl MoveList {
    /// Push a good move to the list.
    pub const fn push_good(&mut self, m: Move, s: i32) {
        debug_assert!(self.good_end <= self.bad_start);
        self.moves[self.good_end] = (m, s);
        self.good_end += 1;
    }

    /// Push a bad move to the list.
    pub const fn push_bad(&mut self, m: Move, s: i32) {
        debug_assert!(self.good_end <= self.bad_start);
        self.moves[self.bad_start] = (m, s);
        self.bad_start -= 1;
    }

    /// Get the next good move.
    pub fn next_good(&mut self) -> Option<Move> {
        if self.good_cur >= self.good_end {
            return None;
        }
        let m = self.take_best(self.good_cur..self.good_end, self.good_cur);
        self.good_cur += 1;
        Some(m)
    }

    /// Get the next bad move.
    pub fn next_bad(&mut self) -> Option<Move> {
        if self.bad_cur <= self.bad_start {
            return None;
        }
        let m = self.take_best(self.bad_start + 1..=self.bad_cur, self.bad_cur);
        self.bad_cur -= 1;
        Some(m)
    }

    /// Partial insertion sort to get the next best move.
    fn take_best(&mut self, range: impl Iterator<Item = usize>, dest: usize) -> Move {
        let best_idx = range.max_by_key(|&i| self.moves[i].1).unwrap();
        self.moves.swap(dest, best_idx);
        self.moves[dest].0
    }
}
