use super::{bitboard::Bitboard, color::Color, rank_file::File, square::Square};

/// Direction enum.
#[rustfmt::skip]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(i8)]
pub enum Direction {
    North =  8,
    East  =  1,
    South = -8,
    West  = -1,

    NorthEast =  9,
    NorthWest =  7,
    SouthEast = -7,
    SouthWest = -9,
}

/// Relative directions.
impl Direction {
    /// Gets the direction up, relative to the color.
    pub const fn up(c: Color) -> Direction {
        match c {
            Color::White => Self::North,
            Color::Black => Self::South,
        }
    }

    /// Gets the direction up + left, relative to the color.
    pub const fn ul(c: Color) -> Direction {
        match c {
            Color::White => Self::NorthWest,
            Color::Black => Self::SouthEast,
        }
    }

    /// Gets the direction up + right, relative to the color.
    pub const fn ur(c: Color) -> Direction {
        match c {
            Color::White => Self::NorthEast,
            Color::Black => Self::SouthWest,
        }
    }
}

/// Move a bitboard by a direction.
impl Bitboard {
    #[rustfmt::skip]
    pub const fn shift(self, direction: Direction) -> Bitboard {
        match direction {
            Direction::North     => Self(self.0 << 8),
            Direction::South     => Self(self.0 >> 8),
            Direction::East      => Self((self.0 & !File::FH.bb().0) << 1),
            Direction::West      => Self((self.0 & !File::FA.bb().0) >> 1),
            Direction::NorthEast => Self((self.0 & !File::FH.bb().0) << 9),
            Direction::NorthWest => Self((self.0 & !File::FA.bb().0) << 7),
            Direction::SouthEast => Self((self.0 & !File::FH.bb().0) >> 7),
            Direction::SouthWest => Self((self.0 & !File::FA.bb().0) >> 9),
        }
    }
}

/// Cast a ray in the given direction.
pub const fn sliding_ray(d: Direction, s: usize, occ: u64) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let mut bb = Bitboard(1u64 << s);

    // We want to stop on the blocker piece, not before it.
    // If the blocker is one of our pieces, it will be filtered out.
    loop {
        bb = bb.shift(d);
        attacks.0 |= bb.0;
        if bb.0 & !occ == 0 {
            break;
        }
    }

    attacks
}

/// Shift a square in the given direction.
impl Square {
    /// Add a direction to a square.
    pub const fn add_dir(self, dir: Direction) -> Self {
        Square::from_raw((self as u8).wrapping_add(dir as u8))
    }

    /// Subtract a direction from a square.
    pub const fn sub_dir(self, dir: Direction) -> Self {
        Square::from_raw((self as u8).wrapping_sub(dir as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_dir() {
        assert_eq!(Square::E4.add_dir(Direction::North), Square::E5);
        assert_eq!(Square::E4.add_dir(Direction::South), Square::E3);
        assert_eq!(Square::E4.add_dir(Direction::East), Square::F4);
        assert_eq!(Square::E4.add_dir(Direction::West), Square::D4);
    }

    #[test]
    fn test_sub_dir() {
        assert_eq!(Square::E4.sub_dir(Direction::North), Square::E3);
        assert_eq!(Square::E4.sub_dir(Direction::South), Square::E5);
        assert_eq!(Square::E4.sub_dir(Direction::East), Square::D4);
        assert_eq!(Square::E4.sub_dir(Direction::West), Square::F4);
    }
}
