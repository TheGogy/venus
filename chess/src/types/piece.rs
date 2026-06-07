use crate::{impl_from_type, types::color::Color};

/// Represents a piece, and is ordered by increasing value.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    #[default]
    None,
}

impl_from_type! {
    Piece, u8, 6
}

/// Represents a piece and a color.
#[rustfmt::skip]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum CPiece {
    WPawn,   BPawn,
    WKnight, BKnight,
    WBishop, BBishop,
    WRook,   BRook,
    WQueen,  BQueen,
    WKing,   BKing,
    #[default]
    None
}

impl_from_type! {
    CPiece, u8, 12
}

impl Piece {
    const UCI_CHAR: &str = "pnbrqk ";

    /// Iterate over all [`Piece`]s.
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..6).map(Self::from_raw)
    }

    /// Get the UCI character for this [`Piece`].
    pub fn to_char(self) -> char {
        Self::UCI_CHAR.chars().nth(self as usize).unwrap_or('?')
    }
}

impl CPiece {
    const UCI_CHAR: &str = "PpNnBbRrQqKk ";

    /// The color of this [`CPiece`].
    pub const fn color(self) -> Color {
        Color::from_raw(self as u8 & 1)
    }

    /// The type of this [`CPiece`].
    pub const fn pt(self) -> Piece {
        Piece::from_raw(self as u8 >> 1)
    }

    /// Create a [`CPiece`] from a [`Color`] and a [`Piece`].
    pub const fn make(c: Color, p: Piece) -> Self {
        Self::from_raw(((p as u8) << 1) + c as u8)
    }

    /// Iterate over all [`CPiece`]s.
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..12).map(Self::from_raw)
    }

    /// Get the UCI character for this [`CPiece`].
    pub fn to_char(self) -> char {
        Self::UCI_CHAR.chars().nth(self as usize).unwrap_or('?')
    }
}

impl TryFrom<char> for CPiece {
    type Error = &'static str;

    /// Constructs a piece from a given character according to UCI specification.
    /// Returns an error (`&' static str`) if the provided `char` does not match any piece.
    fn try_from(value: char) -> Result<Self, Self::Error> {
        Ok(Self::from_raw(Self::UCI_CHAR.chars().position(|x| x == value).ok_or("Invalid CPiece!")? as u8))
    }
}
