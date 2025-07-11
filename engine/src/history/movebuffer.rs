use chess::{MAX_MOVES, types::moves::Move};

/// Temporary buffer to hold the moves in so we can add them to the history.
#[derive(Debug)]
pub struct MoveBuffer {
    pub mvs: [Move; MAX_MOVES],
    pub len: usize,
}

impl Default for MoveBuffer {
    fn default() -> Self {
        Self { mvs: [Move::NULL; MAX_MOVES], len: 0 }
    }
}

impl MoveBuffer {
    /// Add a move to the buffer.
    pub const fn push(&mut self, m: Move) {
        if self.len < MAX_MOVES {
            self.mvs[self.len] = m;
            self.len += 1;
        }
    }
}

/// Iterate through the buffer.
impl<'a> IntoIterator for &'a MoveBuffer {
    type Item = &'a Move;
    type IntoIter = core::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        assert!(self.len <= MAX_MOVES);
        self.mvs[..self.len].iter()
    }
}
