use chess::types::moves::Move;

/// Temporary buffer to hold the moves in so we can add them to the history.
#[derive(Debug, Default)]
pub struct MoveBuffer {
    pub mvs: [Move; Self::SIZE],
    pub len: usize,
}

impl MoveBuffer {
    const SIZE: usize = 30;

    /// Add a move to the buffer.
    pub const fn push(&mut self, m: Move) {
        if self.len < Self::SIZE {
            self.mvs[self.len] = m;
            self.len += 1;
        }
    }

    /// Iterate through the buffer.
    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        self.mvs[..self.len].iter()
    }
}
