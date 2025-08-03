use core::fmt;

use crate::{impl_all_math_ops, impl_math_assign_ops, impl_math_ops};

use super::{
    rank_file::{File, Rank},
    square::Square,
};

/// Bitboard.
/// This is a 64 bit integer that represents an occupancy grid of the chess board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(!0);

    pub const PR: [Self; 2] = [Rank::R7.bb(), Rank::R2.bb()]; // Promotion ranks.
    pub const EP: [Self; 2] = [Rank::R5.bb(), Rank::R4.bb()]; // Enpassant ranks.
    pub const DP: [Self; 2] = [Rank::R3.bb(), Rank::R6.bb()]; // Double push ranks.

    // Light and dark squares.
    pub const WHITE_SQ: Self = Self(0xaa55aa55aa55aa55);
    pub const BLACK_SQ: Self = Self(0x55aa55aa55aa55aa);

    /// If the bitboard is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether the bitboard contains any value.
    pub const fn any(self) -> bool {
        self.0 != 0
    }

    /// Whether the bitboard contains more than one value.
    pub const fn multiple(self) -> bool {
        self.0 & (self.0 - 1) != 0
    }

    /// Set the bit at the given index.
    pub const fn add(&mut self, s: Square) {
        self.0 |= s.bb().0
    }

    /// Pop the bit at the given index.
    pub const fn pop(&mut self, s: Square) {
        self.0 &= !s.bb().0
    }

    /// Get the bit at the current index.
    pub const fn has(self, s: Square) -> bool {
        self.0 & s.bb().0 != 0
    }

    /// Get the least significant bit.
    pub const fn lsb(self) -> Square {
        Square::from_raw(self.0.trailing_zeros() as u8)
    }

    /// Pop the least significant bit.
    pub const fn pop_lsb(&mut self) {
        self.0 &= self.0 - 1
    }

    /// Get the number of bits set.
    pub const fn nbits(self) -> u32 {
        self.0.count_ones()
    }

    /// Performs the given function for each square set in the bitboard.
    pub fn bitloop<F>(&self, mut f: F)
    where
        F: FnMut(Square),
    {
        let mut bb = *self;
        while bb.any() {
            f(bb.lsb());
            bb.pop_lsb();
        }
    }

    /// Get the edge mask for a given square.
    #[rustfmt::skip]
    pub const fn edge_mask(square: Square) -> Self {
        let rank_edges = Rank::R1.bb().0 | Rank::R8.bb().0;
        let file_edges = File::FA.bb().0 | File::FH.bb().0;

        Self((rank_edges & !square.rank().bb().0)
           | (file_edges & !square.file().bb().0),
        )
    }
}

impl_all_math_ops! {
    Bitboard: u64,
    [u64, usize]
}

impl std::ops::Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// Print out a bitboard in a readable way.
impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

        write!(f, "{output}")
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
