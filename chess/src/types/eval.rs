use crate::{MAX_DEPTH, impl_all_math_ops, impl_from_type, impl_math_assign_ops, impl_math_ops};

pub struct Eval(pub i32);

impl Eval {
    pub const DRAW: Eval = Eval(0);
    pub const MATE: Eval = Eval(32000);
    pub const TB_MATE: Eval = Eval(30000);
    pub const INFINITY: Eval = Eval(32001);
    pub const NONE: Eval = Eval(32002);

    pub const LONGEST_MATE: Eval = Eval(Self::MATE.0 - MAX_DEPTH as i32);
    pub const LONGEST_TB_MATE: Eval = Eval(Self::TB_MATE.0 - MAX_DEPTH as i32);

    pub const fn abs(self) -> Self {
        Eval(self.0.abs())
    }

    pub const fn mate_in(ply: usize) -> Self {
        Eval(Self::MATE.0 - ply as i32)
    }

    pub const fn tb_mate_in(ply: usize) -> Self {
        Eval(Self::TB_MATE.0 - ply as i32)
    }

    pub const fn mated_in(ply: usize) -> Self {
        Eval(-Self::MATE.0 + ply as i32)
    }

    pub const fn tb_mated_in(ply: usize) -> Self {
        Eval(-Self::TB_MATE.0 + ply as i32)
    }

    pub const fn is_mate_score(&self) -> bool {
        self.0.abs() >= Self::LONGEST_MATE.0
    }

    pub const fn is_tb_mate_score(&self) -> bool {
        self.0.abs() >= Self::LONGEST_TB_MATE.0 && !self.is_mate_score()
    }

    pub const fn normalized(self) -> Eval {
        const NORMALIZE_PAWN_VALUE: i32 = 199;

        if self.0.abs() >= Self::LONGEST_TB_MATE.0 { self } else { Eval((self.0 * 100) / NORMALIZE_PAWN_VALUE) }
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
