use std::fmt;

use crate::{MAX_DEPTH, impl_all_math_ops, impl_from_type, impl_math_assign_ops, impl_math_ops};

/// Represents the evaluation within a game.
///
/// All valid evaluations are between        [-32000, 32000].
/// All non-terminal evaluations are between [-30000, 30000].
///
/// 0     => draw
/// 32000 => checkmate now
/// 30000 => checkmate according to tablebase
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct Eval(pub i32);

impl Eval {
    pub const DRAW: Eval = Eval(0);
    pub const MATE: Eval = Eval(32000);
    pub const TB_MATE: Eval = Eval(30000);
    pub const INFINITY: Eval = Eval(32001);
    pub const NONE: Eval = Eval(32002);

    pub const LONGEST_MATE: Eval = Eval(Self::MATE.0 - MAX_DEPTH as i32);
    pub const LONGEST_TB_MATE: Eval = Eval(Self::TB_MATE.0 - MAX_DEPTH as i32);

    /// Gets the absolute value of the Eval.
    #[inline]
    pub const fn abs(self) -> Self {
        Eval(self.0.abs())
    }

    /// Gets the max of this eval and another.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Eval(self.0.max(other.0))
    }

    /// Gets the min of this eval and another.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Eval(self.0.min(other.0))
    }

    /// The value of a draw with a bit of randomness to de-incentivise repetitions
    #[inline]
    pub const fn dithered_draw(rand: i32) -> Self {
        let dither_mask = 0x2;
        Eval(Self::DRAW.0 + (rand & dither_mask))
    }

    /// Gets the internal eval representation for checkmate in `ply`.
    #[inline]
    pub const fn mate_in(ply: usize) -> Self {
        Eval(Self::MATE.0 - ply as i32)
    }

    /// Gets the internal eval representation for tablebase checkmate in `ply`.
    #[inline]
    pub const fn tb_mate_in(ply: usize) -> Self {
        Eval(Self::TB_MATE.0 - ply as i32)
    }

    /// Gets the internal eval representation for opponent checkmate in `ply`.
    #[inline]
    pub const fn mated_in(ply: usize) -> Self {
        Eval(-Self::MATE.0 + ply as i32)
    }

    /// Gets the internal eval representation for opponent tablebase checkmate in `ply`.
    #[inline]
    pub const fn tb_mated_in(ply: usize) -> Self {
        Eval(-Self::TB_MATE.0 + ply as i32)
    }

    /// Whether this score implies checkmate.
    #[inline]
    pub const fn is_mate_score(&self) -> bool {
        self.0.abs() >= Self::LONGEST_MATE.0
    }

    /// Whether this score implies checkmate has been found in the tb.
    #[inline]
    pub const fn is_tb_mate_score(&self) -> bool {
        self.0.abs() >= Self::LONGEST_TB_MATE.0
    }

    /// Gets the corrected eval score, forcing between LONGEST_TB_MATE.
    #[inline]
    pub const fn from_corrected(self, ply: usize) -> Self {
        if self.0 >= Eval::LONGEST_TB_MATE.0 {
            Eval(self.0 - ply as i32)
        } else if self.0 <= -Eval::LONGEST_TB_MATE.0 {
            Eval(self.0 + ply as i32)
        } else {
            self
        }
    }

    /// Gets the corrected eval score, incorporating mate scores.
    #[inline]
    pub const fn to_corrected(self, ply: usize) -> Self {
        if self.0 >= Eval::LONGEST_TB_MATE.0 {
            Eval(self.0 + ply as i32)
        } else if self.0 <= -Eval::LONGEST_TB_MATE.0 {
            Eval(self.0 - ply as i32)
        } else {
            self
        }
    }

    #[inline]
    pub const fn normalized(self) -> Eval {
        const NORMALIZE_PAWN_VALUE: i32 = 199;

        if self.is_mate_score() { self } else { Eval((self.0 * 100) / NORMALIZE_PAWN_VALUE) }
    }
}

impl fmt::Display for Eval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_mate_score() {
            let moves_to_mate = (Self::MATE - self.abs() + 1) / 2;
            if *self > Self::DRAW {
                write!(f, "mate {moves_to_mate}")
            } else {
                write!(f, "mate -{moves_to_mate}")
            }
        } else {
            write!(f, "cp {}", self.normalized().0)
        }
    }
}

impl std::ops::Neg for Eval {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl_all_math_ops! {
    Eval: i32,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}

impl_from_type! {
    Eval, i32,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}
