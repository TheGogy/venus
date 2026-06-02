/// Implements math and math-assign operations for a newtype wrapper and its inner primitive.
///
/// Each pair is `Op::op / OpAssign::op_assign`.  You can supply any subset of pairs, or use
/// [`impl_all_math_ops!`] to get the full standard set.
///
/// # Example
/// ```
/// impl_math_ops! {
///     Bitboard: u64,
///     BitAnd::bitand / BitAndAssign::bitand_assign,
///     BitOr::bitor   / BitOrAssign::bitor_assign,
/// }
/// ```
#[macro_export]
macro_rules! impl_math_ops {
    ($t:ty: $inner:ty, $($trait:ident::$fn:ident / $assign:ident::$assign_fn:ident),* $(,)?) => {$(
        impl std::ops::$trait for $t {
            type Output = Self;
            #[inline(always)]
            fn $fn(self, rhs: Self) -> Self { Self(std::ops::$trait::$fn(self.0, rhs.0)) }
        }
        impl std::ops::$trait<$inner> for $t {
            type Output = Self;
            #[inline(always)]
            fn $fn(self, rhs: $inner) -> Self { Self(std::ops::$trait::$fn(self.0, rhs)) }
        }
        impl std::ops::$assign for $t {
            #[inline(always)]
            fn $assign_fn(&mut self, rhs: Self) { std::ops::$assign::$assign_fn(&mut self.0, rhs.0) }
        }
        impl std::ops::$assign<$inner> for $t {
            #[inline(always)]
            fn $assign_fn(&mut self, rhs: $inner) { std::ops::$assign::$assign_fn(&mut self.0, rhs) }
        }
    )*};
}

/// Implements the full set of standard math operations for a newtype wrapper.
///
/// # Example
/// ```
/// impl_all_math_ops!(Eval: i32);
/// ```
#[macro_export]
macro_rules! impl_all_math_ops {
    ($t:ty: $inner:ty) => {
        $crate::impl_math_ops! {
            $t: $inner,
            BitAnd::bitand  / BitAndAssign::bitand_assign,
            BitOr::bitor    / BitOrAssign::bitor_assign,
            BitXor::bitxor  / BitXorAssign::bitxor_assign,
            Shl::shl        / ShlAssign::shl_assign,
            Shr::shr        / ShrAssign::shr_assign,
            Add::add        / AddAssign::add_assign,
            Sub::sub        / SubAssign::sub_assign,
            Mul::mul        / MulAssign::mul_assign,
            Div::div        / DivAssign::div_assign,
        }
    };
}
