use super::moves::Move;

#[derive(Clone, Debug)]
pub struct MoveList {
    pub moves: [Move; Self::SIZE],
    len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self { moves: [Move::default(); Self::SIZE], len: 0 }
    }
}

#[allow(unexpected_cfgs)]
impl MoveList {
    /// From Pleco: https://github.com/pleco-rs/Pleco/blob/6370f65881d681d9283f52045489226e802ab4e5/pleco/src/core/move_list.rs#L62
    /// Aligns the MoveList into values that fill a single cache line.
    #[cfg(target_pointer_width = "128")]
    pub const SIZE: usize = 248;
    #[cfg(target_pointer_width = "64")]
    pub const SIZE: usize = 252;
    #[cfg(target_pointer_width = "32")]
    pub const SIZE: usize = 254;
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "8",))]
    pub const SIZE: usize = 255;

    /// Whether the movelist is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The length of the move list.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Push a move to the move list.
    #[inline]
    pub const fn push(&mut self, m: Move) {
        debug_assert!(self.len < Self::SIZE);
        self.moves[self.len] = m;
        self.len += 1;
    }

    /// Iterate through the move list.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        self.moves[..self.len].iter()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.moves[..self.len].iter()
    }
}
