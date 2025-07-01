use chess::{MAX_MOVES, types::moves::Move};

pub const LEFT: bool = true;
pub const RIGHT: bool = false;

#[derive(Clone, Debug)]
pub struct MoveList {
    // Moves and scores.
    mvs: [Move; MAX_MOVES],
    scs: [i32; MAX_MOVES],

    // Current Index.
    cur: usize,

    // SAFETY: It is guaranteed that left < right, as the list is 220 long and there are a max of
    // 218 moves in a given position.
    left: usize,
    right: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self { mvs: [Move::NONE; MAX_MOVES], scs: [0; MAX_MOVES], cur: 0, right: MAX_MOVES - 1, left: 0 }
    }
}

impl MoveList {
    // Selects the next best move, and moves it to the front of the list.
    // We then move the current index towards the middle of the list.
    pub const fn select_upto<const IS_LEFT: bool>(&mut self) -> Move {
        // Make sure we are not going past the indices!
        assert!((IS_LEFT && self.has_moves::<LEFT>()) || (!IS_LEFT && self.has_moves::<RIGHT>()));
        assert!(self.cur < MAX_MOVES);

        let cur = self.cur;
        let end = if IS_LEFT { self.left } else { self.right };

        let mut best_idx = cur;
        let mut best_scr = self.scs[cur];

        let mut i = cur;
        while i != end {
            assert!(i < MAX_MOVES);
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
            // SAFETY: If we are on the right hand side, self.cur is guaranteed to be > 0.
            self.cur -= 1
        }

        self.mvs[cur]
    }

    /// Swaps moves and scores.
    pub const fn swap(&mut self, a: usize, b: usize) {
        self.mvs.swap(a, b);
        self.scs.swap(a, b);
    }

    /// Adds in a move and score at either the left or the right
    /// and increments / decrements the index.
    pub const fn insert<const IS_LEFT: bool>(&mut self, m: Move, s: i32) {
        let i = if IS_LEFT { self.left } else { self.right };
        self.mvs[i] = m;
        self.scs[i] = s;

        if IS_LEFT {
            self.left += 1;
        } else {
            self.right -= 1;
        }
    }

    /// Whether we have remaining moves on the given side.
    pub const fn has_moves<const IS_LEFT: bool>(&self) -> bool {
        if IS_LEFT { self.cur < self.left } else { self.cur > self.right }
    }

    /// Prepare the current index pointer to go over the bad moves.
    /// This MUST be called before using `select_upto::<RIGHT>()`!!
    pub const fn prepare_bad_moves(&mut self) {
        self.cur = MAX_MOVES - 1;
    }
}
