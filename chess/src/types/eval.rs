#![allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]

use std::fmt;

use crate::{defs::MAX_PLY, impl_all_math_ops};

/// Represents the evaluation within a game.
///
/// All valid evaluations are between        [-32000, 32000].
/// All non-terminal evaluations are between [-30000, 30000].
///
/// 0     => draw
/// 30000 => checkmate according to tablebase
/// 32000 => checkmate now
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct Eval(pub i32);

impl_all_math_ops! (Eval: i32);

impl Eval {
    pub const DRAW: Self = Self(0);
    pub const TB_MATE: Self = Self(31000);
    pub const MATE: Self = Self(32000);
    pub const INFINITY: Self = Self(32001);

    pub const LONGEST_MATE: Self = Self(Self::MATE.0 - MAX_PLY as i32);
    pub const LONGEST_TB_MATE: Self = Self(Self::TB_MATE.0 - MAX_PLY as i32);

    /// Gets the absolute value of the Eval.
    pub const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Gets the max of this eval and another.
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Gets the min of this eval and another.
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Gets the midpoint between two evaluations.
    pub const fn midpoint(a: Self, b: Self) -> Self {
        Self(i32::midpoint(a.0, b.0))
    }

    /// Linearly interpolate between two evaluations.
    pub const fn lerp(a: Self, b: Self, t: f32) -> Self {
        Self(t.mul_add((b.0 - a.0) as f32, a.0 as f32) as i32)
    }

    /// The value of a draw with a bit of randomness to de-incentivise repetitions.
    pub const fn dithered_draw(rand: i32) -> Self {
        let dither_mask = 0b11;
        Self(Self::DRAW.0 + (rand & dither_mask))
    }

    /// Gets the internal eval representation for checkmate in `ply`.
    pub const fn search_mate_in(ply: usize) -> Self {
        Self(Self::MATE.0 - ply as i32)
    }

    /// Gets the internal eval representation for tablebase checkmate in `ply`.
    pub const fn tb_mate_in(ply: usize) -> Self {
        Self(Self::TB_MATE.0 - ply as i32)
    }

    /// Gets the internal eval representation for opponent checkmate in `ply`.
    pub const fn search_mated_in(ply: usize) -> Self {
        Self(-Self::MATE.0 + ply as i32)
    }

    /// Gets the internal eval representation for opponent tablebase checkmate in `ply`.
    pub const fn tb_mated_in(ply: usize) -> Self {
        Self(-Self::TB_MATE.0 + ply as i32)
    }

    /// Whether this score implies a win.
    pub const fn is_search_win(self) -> bool {
        self.0 >= Self::LONGEST_MATE.0
    }

    /// Whether this score implies a loss.
    pub const fn is_search_loss(self) -> bool {
        self.0 <= -Self::LONGEST_MATE.0
    }

    /// Whether this score implies a win.
    pub const fn is_win(self) -> bool {
        self.0 >= Self::LONGEST_TB_MATE.0
    }

    /// Whether this score implies a loss.
    pub const fn is_loss(self) -> bool {
        self.0 <= -Self::LONGEST_TB_MATE.0
    }

    /// Whether this score implies either side has a proven mate.
    pub const fn is_terminal(self) -> bool {
        self.is_win() || self.is_loss()
    }

    /// Whether or not this is a valid score.
    pub const fn is_valid(&self) -> bool {
        self.0.abs() < Self::INFINITY.0
    }

    /// Gets the eval from the corrected value stored in the TT.
    pub const fn from_tb_score(self, ply: usize) -> Self {
        if self.is_win() {
            Self(self.0 - ply as i32)
        } else if self.is_loss() {
            Self(self.0 + ply as i32)
        } else {
            self
        }
    }

    /// Converts the eval to the corrected value stored in the TT.
    pub const fn to_tb_score(self, ply: usize) -> Self {
        if self.is_win() {
            Self(self.0 + ply as i32)
        } else if self.is_loss() {
            Self(self.0 - ply as i32)
        } else {
            self
        }
    }

    /// Normalizes the evaluation.
    /// TODO: Feed it more games
    /// <https://github.com/official-stockfish/WDL_model>
    pub const fn to_centipawns(self) -> i32 {
        const NORMALIZE_PAWN_VALUE: i32 = 168;

        if !self.is_terminal() { (self.0 * 100) / NORMALIZE_PAWN_VALUE } else { self.0 }
    }

    /// Clamps eval to the valid (non-terminal) range.
    pub fn clamp_to_nonterminal(self) -> Self {
        Self(self.0.clamp(-Self::LONGEST_TB_MATE.0 + 1, Self::LONGEST_TB_MATE.0 - 1))
    }
}

/// Display the eval according to UCI format.
impl fmt::Display for Eval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_terminal() {
            write!(f, "cp {}", self.to_centipawns())
        } else {
            let moves_to_mate = (Self::MATE.0 - self.abs().0 + 1) / 2;
            let sign = if *self > Self::DRAW { "" } else { "-" };
            write!(f, "mate {sign}{moves_to_mate}")
        }
    }
}

impl std::ops::Neg for Eval {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}
