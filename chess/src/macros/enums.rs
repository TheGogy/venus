/// Enables conversion from all given types to given type.
///
/// # Example
///
/// impl_from_type! {
///     Square, u8,
///     [i64, i32, i16, i8, u64, u32, u16, u8, usize]
/// }
#[macro_export]
macro_rules! impl_from_type {
    ($t:ty, $inner:ty, $max:expr, [$($from:ty),*]) => {
        $(impl From<$from> for $t {
            fn from(value: $from) -> Self {
                unsafe { std::mem::transmute(value as $inner) }
            }
        })*

        impl $t {
            // Safety: caller guarantees this is within bounds.
            pub const fn from_index(i: usize) -> Self {
                unsafe { std::mem::transmute(i as $inner) }
            }

            // Safety: caller guarantees this is within bounds.
            pub const fn from_raw(i: $inner) -> Self {
                unsafe { std::mem::transmute(i as $inner) }
            }
        }
    };
}

/// Enables creation and indexing of lists with this enum.
///
/// # Example
///
/// impl_lists! {
///     Square, 64
/// }
#[macro_export]
macro_rules! impl_lists {
    ($t:ty, $num:expr) => {
        impl $t {
            pub const NUM: usize = $num;

            pub const fn idx(self) -> usize {
                self as usize
            }
        }
    };
}
