use chess::{defs::MAX_MOVES, types::moves::Move};

#[derive(Clone, Debug)]
pub struct MoveList {
    moves: [(Move, i32); MAX_MOVES],

    cur: usize,
    good_end: usize,
    bad_start: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self { moves: [(Move::NONE, 0); MAX_MOVES], cur: 0, good_end: 0, bad_start: MAX_MOVES - 1 }
    }
}

impl MoveList {
    pub const fn push_good(&mut self, m: Move, s: i32) {
        self.moves[self.good_end] = (m, s);
        self.good_end += 1;
    }

    pub const fn push_bad(&mut self, m: Move, s: i32) {
        self.moves[self.bad_start] = (m, s);
        self.bad_start -= 1;
    }

    pub fn next_good(&mut self) -> Option<Move> {
        if self.cur >= self.good_end {
            return None;
        }

        Some(self.select_best(self.cur, self.good_end))
    }

    pub fn next_bad(&mut self) -> Option<Move> {
        if self.cur <= self.bad_start {
            return None;
        }

        let cur = self.cur;
        let best_idx = (self.bad_start + 1..=cur).max_by_key(|&i| self.moves[i].1).unwrap();
        self.moves.swap(cur, best_idx);
        self.cur -= 1;
        Some(self.moves[cur].0)
    }

    pub const fn prepare_bad_moves(&mut self) {
        self.cur = MAX_MOVES - 1;
    }

    fn select_best(&mut self, start: usize, end: usize) -> Move {
        let best_idx = (start..end).max_by_key(|&i| self.moves[i].1).unwrap();
        self.moves.swap(start, best_idx);
        self.cur += 1;
        self.moves[start].0
    }
}
