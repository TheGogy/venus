use core::fmt;
use std::str::FromStr;

use crate::{impl_from_type, impl_lists};

use super::{
    bitboard::Bitboard,
    color::Color,
    rank_file::{File, Rank},
};

/// Square enum.
///
/// A1 ... H8
#[rustfmt::skip]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
    #[default]
    Invalid
}

impl Square {
    /// Make a square from a rank and file.
    pub const fn make(r: Rank, f: File) -> Self {
        Self::from_raw(((r as u8) << 3) + f as u8)
    }

    /// Get a bitboard with only this square set.
    pub const fn bb(self) -> Bitboard {
        Bitboard(1u64 << self as u64)
    }

    /// Get the file the square is on.
    pub const fn file(self) -> File {
        File::from_raw(self as u8 & 0x7)
    }

    /// Get the rank the square is on.
    pub const fn rank(self) -> Rank {
        Rank::from_raw(self as u8 >> 3)
    }

    /// Gets the square relative to white's side.
    pub const fn relative(self, c: Color) -> Self {
        Self::from_raw(self as u8 ^ (c as u8 * 56))
    }

    /// Moves the square forward by one relative to the side.
    pub const fn forward(self, c: Color) -> Self {
        Self::from_raw(self as u8 + 8 - (16 * c.idx() as u8))
    }

    /// Gets the next square. (A1 -> H1 -> A8 -> H8).
    pub const fn next(self) -> Self {
        Self::from_raw(self as u8 + 1)
    }

    /// Gets the previous square. (H8 -> A8 -> H1 -> A1).
    pub const fn prev(self) -> Self {
        Self::from_raw(self as u8 - 1)
    }

    /// Iterate over all squares.
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..64).map(Self::from_raw)
    }

    /// Flip horizontal.
    pub const fn fliph(self) -> Self {
        Self::from_raw(self as u8 ^ 7)
    }

    /// Flip vertical.
    pub const fn flipv(self) -> Self {
        Self::from_raw(self as u8 ^ 56)
    }
}

impl_lists! {Square, 64}

/// Convert a string to a Square
impl FromStr for Square {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 2 {
            return Err("Invalid square format");
        }

        let mut chars = s.chars();
        let file = chars.next().ok_or("Missing file")?;
        let rank = chars.next().ok_or("Missing rank")?;

        if !('a'..='h').contains(&file) || !('1'..='8').contains(&rank) {
            return Err("Invalid file or rank");
        }

        let file_idx = (file as u8) - b'a';
        let rank_idx = (rank as u8) - b'1';
        let square_idx = rank_idx * 8 + file_idx;

        Ok(Self::from_raw(square_idx))
    }
}

/// Display a square.
impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file() as u8;
        let rank = self.rank() as u8;
        write!(f, "{}{}", (b'a' + file) as char, (b'1' + rank) as char)
    }
}

impl_from_type! {
    Square, u8, 64,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_prev() {
        assert_eq!(Square::A1.next(), Square::B1);
        assert_eq!(Square::B3.prev(), Square::A3);
    }
}
