use chess::types::eval::Eval;

/// TT Bound.
/// Upper: search at this position fails high.
/// Lower: search at this position fails low.
/// Exact: exact value of this node.
#[rustfmt::skip]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
#[repr(u8)]
pub enum Bound {
    #[default]
    None  = 0b00,
    Upper = 0b01,
    Lower = 0b10,
    Exact = 0b11,
}

impl Bound {
    /// Whether this bound contains the other bound.
    pub const fn has(self, other: Bound) -> bool {
        self as u8 & other as u8 != 0
    }

    /// Whether the given eval is usable given the operand.
    pub const fn is_usable(self, eval: Eval, operand: Eval) -> bool {
        self.has(if eval.0 >= operand.0 { Self::Lower } else { Self::Upper })
    }
}

/// Wrapper to put the age, pv and bound into one u8.
///
/// 00000011 - Bound
/// 00000100 - Is_PV
/// 11111000 - Age
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
pub struct AgePVBound(u8);

// The max age for any tt entry.
pub const MAX_AGE: u8 = 1 << 5;

impl AgePVBound {
    pub const fn from(bound: Bound, is_pv: bool, table_age: u8) -> Self {
        Self(bound as u8 | (is_pv as u8) << 2 | table_age << 3)
    }

    pub const fn bound(self) -> Bound {
        unsafe { std::mem::transmute(self.0 & 0b11) }
    }

    pub const fn is_pv(self) -> bool {
        self.0 & 0b100 != 0
    }

    pub const fn age(self) -> u8 {
        self.0 >> 3
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub const fn relative_age(self, table_age: u8) -> u8 {
        (MAX_AGE + table_age - self.age()) % MAX_AGE
    }
}
