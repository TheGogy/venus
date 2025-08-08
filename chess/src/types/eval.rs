use std::fmt;

use crate::{defs::MAX_PLY, impl_all_math_ops, impl_math_assign_ops, impl_math_ops};

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

impl Eval {
    pub const DRAW: Eval = Eval(0);
    pub const TB_MATE: Eval = Eval(30000);
    pub const MATE: Eval = Eval(32000);
    pub const INFINITY: Eval = Eval(32001);

    pub const LONGEST_MATE: Eval = Eval(Self::MATE.0 - MAX_PLY as i32);
    pub const LONGEST_TB_MATE: Eval = Eval(Self::TB_MATE.0 - MAX_PLY as i32);

    /// Gets the absolute value of the Eval.
    #[inline(always)]
    pub const fn abs(self) -> Self {
        Eval(self.0.abs())
    }

    /// Gets the max of this eval and another.
    #[inline(always)]
    pub fn max(self, other: Self) -> Self {
        Eval(self.0.max(other.0))
    }

    /// Gets the min of this eval and another.
    #[inline(always)]
    pub fn min(self, other: Self) -> Self {
        Eval(self.0.min(other.0))
    }

    /// The value of a draw with a bit of randomness to de-incentivise repetitions.
    #[inline(always)]
    pub const fn dithered_draw(rand: i32) -> Self {
        let dither_mask = 0b11;
        Eval(Self::DRAW.0 + (rand & dither_mask))
    }

    /// Gets the internal eval representation for checkmate in `ply`.
    #[inline(always)]
    pub const fn search_mate_in(ply: usize) -> Self {
        Eval(Self::MATE.0 - ply as i32)
    }

    /// Gets the internal eval representation for tablebase checkmate in `ply`.
    #[inline(always)]
    pub const fn mate_in(ply: usize) -> Self {
        Eval(Self::TB_MATE.0 - ply as i32)
    }

    /// Gets the internal eval representation for opponent checkmate in `ply`.
    #[inline(always)]
    pub const fn search_mated_in(ply: usize) -> Self {
        Eval(-Self::MATE.0 + ply as i32)
    }

    /// Gets the internal eval representation for opponent tablebase checkmate in `ply`.
    #[inline(always)]
    pub const fn mated_in(ply: usize) -> Self {
        Eval(-Self::TB_MATE.0 + ply as i32)
    }

    /// Whether this score implies a win.
    #[inline(always)]
    pub const fn is_search_win(self) -> bool {
        self.0 >= Self::LONGEST_MATE.0
    }

    /// Whether this score implies a loss.
    #[inline(always)]
    pub const fn is_search_loss(self) -> bool {
        self.0 <= -Self::LONGEST_MATE.0
    }

    /// Whether this score implies a win.
    #[inline(always)]
    pub const fn is_win(self) -> bool {
        self.0 >= Self::LONGEST_TB_MATE.0
    }

    /// Whether this score implies a loss.
    #[inline(always)]
    pub const fn is_loss(self) -> bool {
        self.0 <= -Self::LONGEST_TB_MATE.0
    }

    /// Whether this score implies checkmate.
    #[inline(always)]
    pub const fn is_mate(self) -> bool {
        self.0.abs() >= Self::LONGEST_MATE.0
    }

    /// Whether this score implies that the game has not been confirmed as mate.
    #[inline(always)]
    pub const fn nonterminal(self) -> bool {
        self.0.abs() < Self::LONGEST_TB_MATE.0
    }

    /// Whether or not this is a valid score.
    #[inline(always)]
    pub const fn is_valid(&self) -> bool {
        self.0.abs() < Self::INFINITY.0
    }

    /// Gets the eval from the corrected value stored in the TT.
    #[inline(always)]
    pub const fn from_corrected(self, ply: usize) -> Self {
        if self.is_win() {
            Eval(self.0 - ply as i32)
        } else if self.is_loss() {
            Eval(self.0 + ply as i32)
        } else {
            self
        }
    }

    /// Converts the eval to the corrected value stored in the TT.
    #[inline(always)]
    pub const fn to_corrected(self, ply: usize) -> Self {
        if self.is_win() {
            Eval(self.0 + ply as i32)
        } else if self.is_loss() {
            Eval(self.0 - ply as i32)
        } else {
            self
        }
    }

    /// Normalizes the evaluation.
    /// TODO: Feed it more games
    ///       It needs more games
    ///       It always needs more games
    /// https://github.com/official-stockfish/WDL_model
    #[inline(always)]
    pub const fn normalized(self) -> i32 {
        const NORMALIZE_PAWN_VALUE: i32 = 168;

        if self.is_mate() { self.0 } else { (self.0 * 100) / NORMALIZE_PAWN_VALUE }
    }

    /// Clamps eval to the valid (non-terminal) range.
    #[inline(always)]
    pub fn clamped(self) -> Eval {
        Eval(self.0.clamp(-Self::LONGEST_TB_MATE.0 + 1, Self::LONGEST_TB_MATE.0 - 1))
    }
}

/// Display the eval according to UCI format.
impl fmt::Display for Eval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_mate() {
            let moves_to_mate = (Self::MATE.0 - self.abs().0 + 1) / 2;
            let sign = if *self > Self::DRAW { "" } else { "-" };
            write!(f, "mate {sign}{moves_to_mate}")
        } else {
            write!(f, "cp {}", self.normalized())
        }
    }
}

impl std::ops::Neg for Eval {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl_all_math_ops! {
    Eval: i32,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}
