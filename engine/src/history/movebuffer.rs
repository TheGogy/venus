use chess::types::moves::Move;

const SIZE: usize = 32;

/// Move buffer.
/// This is used to contain a buffer of the moves searched.
#[derive(Default, Debug)]
pub struct MoveBuffer {
    pub moves: [Move; 32],
    len: usize,
}

impl MoveBuffer {
    #[inline]
    pub const fn push(&mut self, m: Move) {
        if self.len < SIZE {
            self.moves[self.len] = m;
            self.len += 1;
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }
}

impl<'a> IntoIterator for &'a MoveBuffer {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.moves[..self.len].iter()
    }
}
