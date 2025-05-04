use std::{fmt, ops::Not};

use crate::impl_from_type;

/// Color. This represents the two sides, White and Black.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    /// Total number of colors.
    pub const NUM: usize = 2;

    /// The index of this color.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Iterate over both colors.
    #[inline]
    pub fn iter() -> impl Iterator<Item = Color> {
        [Color::White, Color::Black].into_iter()
    }
}

/// Toggle the current color.
impl Not for Color {
    type Output = Color;

    fn not(self) -> Self {
        Self::from_raw(1 ^ (self as u8))
    }
}

impl_from_type! {
    Color, u8,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize, bool]
}

/// Get a color from a character.
/// 'w' => Color::White,
/// 'b' => Color::Black
impl TryFrom<char> for Color {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'w' => Ok(Self::White),
            'b' => Ok(Self::Black),
            _ => Err("Invalid color!"),
        }
    }
}

/// Display the color.
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::White => write!(f, "w"),
            Self::Black => write!(f, "b"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_opp() {
        assert_eq!(!Color::White, Color::Black);
        assert_eq!(!Color::Black, Color::White);
    }

    #[test]
    fn test_color_from() {
        assert_eq!(Color::try_from('w').unwrap(), Color::White);
        assert_eq!(Color::try_from('b').unwrap(), Color::Black);
        assert_eq!(Color::try_from('b').unwrap(), Color::Black);
    }

    #[test]
    fn test_color_display() {
        assert_eq!(Color::White.to_string(), "w");
        assert_eq!(Color::Black.to_string(), "b");
    }
}
