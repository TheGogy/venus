/// Implements math operations for a given type, and all operations with a primitive.
///
/// # Examples
///
/// impl_math_ops! {
///     Bitboard,
///     BitAnd::bitand,
///     BitOr::bitor,
///     BitXor::bitxor
/// }
///
/// impl_math_ops! {
///     Bitboard: u64
///     u64,
///     BitAnd::bitand,
///     BitOr::bitor,
///     BitXor::bitxor
/// }
#[macro_export]
macro_rules! impl_math_ops {
    ($t:ty, $($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait for $t {
            type Output = Self;

            #[inline]
            fn $fn(self, other: Self) -> Self::Output {
                Self(std::ops::$trait::$fn(self.0, other.0))
            }
        })*
    };

    ($t:ty: $inner:ty, $prim:ty, $($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait<$prim> for $t {
            type Output = $t;

            #[inline]
            fn $fn(self, other: $prim) -> $t {
                unsafe { std::mem::transmute(std::ops::$trait::$fn(self.0, other as $inner)) }
            }
        })*

        $(impl std::ops::$trait<$t> for $prim {
            type Output = $prim;

            #[inline]
            fn $fn(self, other: $t) -> $prim {
                #[allow(clippy::useless_transmute)]
                unsafe { std::mem::transmute(std::ops::$trait::$fn(self, other.0 as $prim)) }
            }
        })*
    };
}

/// Implements math assignment operations for a given type, and all operations with a primitive.
///
/// # Examples
///
/// impl_math_assign_ops! {
///     Bitboard,
///     BitAndAssign::bitand_assign,
///     BitOrAssign::bitor_assign,
///     BitXorAssign::bitxor_assign
/// }
///
/// impl_math_assign_ops! {
///     Bitboard: u64
///     u64,
///     BitAndAssign::bitand_assign,
///     BitOrAssign::bitor_assign,
///     BitXorAssign::bitxor_assign
/// }
#[macro_export]
macro_rules! impl_math_assign_ops {
    ($t:ty, $($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait for $t {

            #[inline]
            fn $fn(&mut self, other: Self) {
                std::ops::$trait::$fn(&mut self.0, other.0)
            }
        })*
    };

    ($t:ty: $inner:ty, $prim:ty, $($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait<$prim> for $t {

            #[inline]
            fn $fn(&mut self, other: $prim) {
                std::ops::$trait::$fn(&mut self.0, other as $inner)
            }
        })*
    };
}

/// Implement all math and math assign operations between a type and some primitives.
///
/// # Example:
///
/// impl_all_math_ops {
///     Eval: i32,
///     [u64, u32, u16, u8, i64, i32, i16, i8]
/// }
#[macro_export]
macro_rules! impl_all_math_ops {
    ($t:ty: $inner:ty, [$($prim:ty),*]) => {
        impl_math_ops! {
            $t,
            BitAnd::bitand, BitOr::bitor, BitXor::bitxor,
            Shl::shl, Shr::shr,
            Add::add, Sub::sub, Mul::mul, Div::div
        }

        impl_math_assign_ops! {
            $t,
            BitAndAssign::bitand_assign, BitOrAssign::bitor_assign, BitXorAssign::bitxor_assign,
            ShlAssign::shl_assign, ShrAssign::shr_assign,
            AddAssign::add_assign, SubAssign::sub_assign, MulAssign::mul_assign, DivAssign::div_assign
        }

        $(
            impl_math_ops! {
                $t: $inner,
                $prim,
                BitAnd::bitand, BitOr::bitor, BitXor::bitxor,
                Shl::shl, Shr::shr,
                Add::add, Sub::sub, Mul::mul, Div::div
            }

            impl_math_assign_ops! {
                $t: $inner,
                $prim,
                BitAndAssign::bitand_assign, BitOrAssign::bitor_assign, BitXorAssign::bitxor_assign,
                ShlAssign::shl_assign, ShrAssign::shr_assign,
                AddAssign::add_assign, SubAssign::sub_assign, MulAssign::mul_assign, DivAssign::div_assign
            }
        )*
    };
}
