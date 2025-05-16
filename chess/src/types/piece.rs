use crate::impl_from_type;

use super::color::Color;

/// Piece.
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

/// CPiece.
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

impl Piece {
    pub const NUM: usize = 6;
    const UCI_CHAR: &str = "pnbrqk ";

    /// The index of this piece.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Iterate over all Pieces.
    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..6).map(Self::from_raw)
    }

    /// Get the UCI character for this Piece.
    pub fn to_char(self) -> char {
        Self::UCI_CHAR.chars().nth(self as usize).expect("Invalid piece!")
    }
}

impl CPiece {
    pub const NUM: usize = 12;
    const UCI_CHAR: &str = "PpNnBbRrQqKk ";

    /// The index of this CPiece.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The color of this CPiece.
    #[inline]
    pub const fn color(self) -> Color {
        Color::from_raw(self as u8 & 1)
    }

    /// The type of this CPiece.
    #[inline]
    pub const fn pt(self) -> Piece {
        Piece::from_raw(self as u8 >> 1)
    }

    /// Create a CPiece from a Color and a Piece.
    #[inline]
    pub const fn create(c: Color, p: Piece) -> Self {
        Self::from_raw(((p as u8) << 1) + c as u8)
    }

    /// Iterate over all CPieces.
    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..12).map(Self::from_raw)
    }

    /// Get the UCI character for this CPiece.
    pub fn to_char(self) -> char {
        Self::UCI_CHAR.chars().nth(self as usize).expect("Invalid CPiece!")
    }
}

impl TryFrom<char> for CPiece {
    type Error = &'static str;

    /// Constructs a piece from a given character according to UCI specification.
    /// Returns an error (`&' static str`) if the provided `char` does not match any piece.
    fn try_from(value: char) -> Result<Self, Self::Error> {
        Ok(Self::from(Self::UCI_CHAR.chars().position(|x| x == value).ok_or("Invalid CPiece!")?))
    }
}

impl_from_type! {
    Piece, u8,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}

impl_from_type! {
    CPiece, u8,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_index() {
        for i in 0..Piece::NUM {
            assert_eq!(Piece::from(i).index(), i);
        }
    }

    #[test]
    fn test_cpiece_index() {
        for i in 0..CPiece::NUM {
            assert_eq!(CPiece::from(i).index(), i);
        }
    }

    #[test]
    fn test_cpiece_color() {
        assert_eq!(CPiece::WPawn.color(), Color::White);
        assert_eq!(CPiece::BQueen.color(), Color::Black);
    }

    #[test]
    fn test_cpiece_type() {
        assert_eq!(CPiece::WPawn.pt(), Piece::Pawn);
        assert_eq!(CPiece::BQueen.pt(), Piece::Queen);
    }
}
