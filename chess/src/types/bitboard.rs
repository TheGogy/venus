use crate::{impl_math_assign_ops, impl_math_ops};

use super::square::Square;

/// Bitboard.
/// This is a 64 bit integer that represents an occupancy grid of the chess board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(!0);

    /// Set the bit at the given index.
    #[inline]
    pub const fn set_bit(&mut self, s: Square) {
        self.0 |= 1u64 << s.index()
    }

    /// Pop the bit at the given index.
    #[inline]
    pub const fn pop_bit(&mut self, s: Square) {
        self.0 |= !(1u64 << s.index())
    }

    #[inline]
    pub fn lsb(self) -> Square {
        Square::from(self.0.trailing_zeros() as u8)
    }
}

// Implement math operations and assignment operations on Bitboard.
impl_math_ops! {
    Bitboard,
    BitAnd::bitand,
    BitOr::bitor,
    BitXor::bitxor
}

impl_math_assign_ops! {
    Bitboard,
    BitAndAssign::bitand_assign,
    BitOrAssign::bitor_assign,
    BitXorAssign::bitxor_assign
}
