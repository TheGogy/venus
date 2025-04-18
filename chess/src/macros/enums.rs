/// Implements math operations for a given type.
///
/// # Example
///
/// impl_math_ops! {
///     Bitboard,
///     BitAnd::bitand,
///     BitOr::bitor,
///     BitXor::bitxor
/// }
#[macro_export]
macro_rules! impl_math_ops {
    ($t:ty, $($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait for $t {
            type Output = Self;

            fn $fn(self, other: Self) -> Self::Output {
                Self(std::ops::$trait::$fn(self.0, other.0))
            }
        })*
    };
}

/// Implements math assignment operations for a given type.
///
/// # Example
///
/// impl_math_ops! {
///     Bitboard,
///     BitAndAssign::bitand_assign,
///     BitOrAssign::bitor_assign,
///     BitXorAssign::bitxor_assign
/// }
#[macro_export]
macro_rules! impl_math_assign_ops {
    ($t:ty, $($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait for $t {

            fn $fn(&mut self, other: Self) {
                std::ops::$trait::$fn(&mut self.0, other.0)
            }
        })*
    };
}

/// Enables conversion from all given types to given type.
///
/// # Example
///
/// impl_from_type! {
///     Square, u8,
///     u8,
///     usize
/// }
#[macro_export]
macro_rules! impl_from_type {
    ($t:ty, $inner:ty, $($from:ty),*) => {
        $(impl From<$from> for $t {

            fn from(value: $from) -> Self {
                unsafe { std::mem::transmute(value as $inner) }
            }
        })*
    };
}
