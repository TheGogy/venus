use crate::impl_from_type;

use super::{bitboard::Bitboard, color::Color};

/// File enum.
///
/// These are the columns on the chess board.
#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum File {
    FA, FB, FC, FD, FE, FF, FG, FH
}

/// Rank enum.
///
/// These are the rows on the chess board.
#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Rank {
    R1, R2, R3, R4, R5, R6, R7, R8
}

/// Get a File from a character.
impl TryFrom<char> for File {
    type Error = &'static str;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        let index = (c.to_ascii_uppercase() as u8).wrapping_sub(b'A');
        match index {
            0..7 => Ok(File::from(index)),
            _ => Err("Unknown file!"),
        }
    }
}

impl_from_type! {
    File, u8,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}

/// Get a Rank from a character.
impl TryFrom<char> for Rank {
    type Error = &'static str;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        let index = (c as u8).wrapping_sub(b'1');
        match index {
            0..7 => Ok(Rank::from(index)),
            _ => Err("Unknown file!"),
        }
    }
}

impl_from_type! {
    Rank, u8,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}

/// File implemenatations.
impl File {
    /// Directly convert a file to a bitboard.
    #[inline]
    pub const fn to_bb(self) -> Bitboard {
        Bitboard(0x0101010101010101 << (self as u8))
    }

    /// Get the index of the File.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Get the char representing the file.
    pub fn to_char(self) -> char {
        char::from(self as u8 + b'A')
    }
}

/// Rank implemenatations.
impl Rank {
    /// Directly convert a rank to a bitboard.
    #[inline]
    pub const fn bb(self) -> Bitboard {
        Bitboard(0xff << (8 * self as u8))
    }

    /// Get the index of the rank.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Get the file from that color's perspective.
    /// e.g:
    /// File::A.relative(Color::Black) == File::H
    #[inline]
    pub const fn relative(self, c: Color) -> Self {
        Self::from_raw(self as u8 ^ (c as u8 * 7))
    }

    /// Get the char representing the rank.
    pub fn to_char(self) -> char {
        char::from(self as u8 + b'1')
    }
}
