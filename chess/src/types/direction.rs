use super::{bitboard::Bitboard, color::Color, rank_file::File};

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
    #[inline]
    pub const fn forward(c: Color) -> Direction {
        match c {
            Color::White => Self::North,
            Color::Black => Self::South,
        }
    }

    #[inline]
    pub const fn up_left(c: Color) -> Direction {
        match c {
            Color::White => Self::NorthWest,
            Color::Black => Self::SouthEast,
        }
    }

    #[inline]
    pub const fn up_right(c: Color) -> Direction {
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
            Direction::East      => Self((self.0 & !File::FH.to_bb().0) << 1),
            Direction::West      => Self((self.0 & !File::FA.to_bb().0) >> 1),
            Direction::NorthEast => Self((self.0 & !File::FH.to_bb().0) << 9),
            Direction::NorthWest => Self((self.0 & !File::FA.to_bb().0) << 7),
            Direction::SouthEast => Self((self.0 & !File::FH.to_bb().0) >> 7),
            Direction::SouthWest => Self((self.0 & !File::FA.to_bb().0) >> 9),
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
