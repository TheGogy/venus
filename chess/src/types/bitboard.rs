use core::fmt;

use crate::{impl_math_assign_ops, impl_math_ops};

use super::{
    rank_file::{File, Rank},
    square::Square,
};

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

    /// Get the bit at the current index.
    #[inline]
    pub const fn get_bit(self, s: Square) -> bool {
        self.0 & 1u64 << s.index() != 0
    }

    /// Get the least significant bit.
    #[inline]
    pub fn lsb(self) -> Square {
        Square::from(self.0.trailing_zeros() as u8)
    }

    /// Get the edge mask for a given square.
    #[rustfmt::skip]
    pub const fn edge_mask(square: Square) -> Self {
        let rank_edges = Rank::R1.to_bb().0 | Rank::R8.to_bb().0;
        let file_edges = File::FA.to_bb().0 | File::FH.to_bb().0;

        Self((rank_edges & !square.rank().to_bb().0)
           | (file_edges & !square.file().to_bb().0),
        )
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

impl std::ops::Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// Print out a bitboard in a readable way.
impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::new();
        for rank in (0..8).rev() {
            output.push_str(&(rank + 1).to_string()); // Rank label
            output.push(' ');

            for file in 0..8 {
                let square = rank * 8 + file;
                let bit = (self.0 >> square) & 1;
                let symbol = if bit == 1 { "X" } else { "." };
                output.push_str(symbol);
                output.push(' ');
            }

            output.push('\n');
        }
        output.push_str("  a b c d e f g h\n"); // Column labels

        write!(f, "{}", output)
    }
}

// Macro to help with debugging bitboards.
#[macro_export]
macro_rules! assert_bitboard_eq {
    ($left:expr, $right:expr) => {{
        if $left != $right {
            panic!(
                "\nAssertion failed at {}:{}\nFailed: assert_bitboard_eq!({}, {})\n\nExpected:\n{}\nGot:\n{}\n",
                file!(),
                line!(),
                stringify!($left),
                stringify!($right),
                $right,
                $left
            );
        }
    }};
}
