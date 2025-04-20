use core::fmt;
use std::str::FromStr;

use crate::impl_from_type;

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
    pub const NUM: usize = 64;

    /// Make a square from a rank and file
    #[inline]
    pub const fn make(r: Rank, f: File) -> Self {
        unsafe { std::mem::transmute(((r as u8) << 3) + f as u8) }
    }

    /// Get the index of the square.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Get a bitboard with only this square set.
    #[inline]
    pub const fn bb(self) -> Bitboard {
        Bitboard(1u64 << self as u64)
    }

    /// Get the file the square is on.
    #[inline]
    pub const fn file(self) -> File {
        unsafe { std::mem::transmute(self as u8 & 0x7) }
    }

    /// Get the rank the square is on.
    #[inline]
    pub const fn rank(self) -> Rank {
        unsafe { std::mem::transmute(self as u8 >> 3) }
    }

    /// Gets the square relative to white's side.
    #[inline]
    pub const fn relative(self, c: Color) -> Self {
        unsafe { std::mem::transmute(self as u8 ^ (c as u8 * 56)) }
    }

    /// Moves the square forward by one relative to the side.
    #[inline]
    pub const fn forward(self, c: Color) -> Self {
        unsafe { std::mem::transmute(self as i8 + (8 * -(c.index() as i8))) }
    }

    /// Gets the next square. (A1 -> H1 -> A8 -> H8)
    #[inline]
    pub const fn next(self) -> Self {
        unsafe { std::mem::transmute(self as u8 + 1) }
    }

    /// Gets the previous square. (H8 -> A8 -> H1 -> A1)
    #[inline]
    pub const fn prev(self) -> Self {
        unsafe { std::mem::transmute(self as u8 - 1) }
    }

    /// Iterate over all squares.
    #[inline]
    pub fn iter() -> impl Iterator<Item = Square> {
        (0..64).map(Square::from)
    }
}

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

        Ok(unsafe { std::mem::transmute::<u8, Square>(square_idx) })
    }
}

/// Display a square.
impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let file = (*self as u8) & 7;
        let rank = (*self as u8) >> 3;
        write!(f, "{}{}", (b'a' + file) as char, (b'1' + rank) as char)
    }
}

impl_from_type! {
    Square, u8,
    u8,
    u16,
    u32,
    u64,
    i16,
    i32,
    i64,
    usize
}
