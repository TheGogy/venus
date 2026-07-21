use std::{fmt, ops::Not};

use crate::impl_from_type;

/// Color. This represents the two sides, White and Black.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl_from_type! {
    Color, u8, 2
}

impl Color {
    /// Iterate over both colors.
    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::White, Self::Black].into_iter()
    }
}

/// Toggle the current color.
impl Not for Color {
    type Output = Self;

    fn not(self) -> Self {
        Self::from_raw(1 ^ self.to_raw())
    }
}

/// Get a color from a character.
/// 'w' => [`Color::White`],
/// 'b' => [`Color::Black`]
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
