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
            #[inline]
            fn from(value: $from) -> Self {
                unsafe { std::mem::transmute(value as $inner) }
            }
        })*

        impl $t {
            #[inline]
            pub const fn from_index(i: usize) -> Self {
                if i >= $max {
                    // SAFETY: will only ever be used on valid input.
                    unsafe { std::hint::unreachable_unchecked() }
                }
                unsafe { std::mem::transmute(i as $inner) }
            }

            #[inline]
            pub const fn from_raw(i: $inner) -> Self {
                if (i as usize) >= $max {
                    // SAFETY: will only ever be used on valid input.
                    unsafe { std::hint::unreachable_unchecked() }
                }
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

            #[inline]
            pub const fn idx(self) -> usize {
                let idx = self as usize;
                debug_assert!(idx < Self::NUM);
                // Safety: caller guarantees idx is always < NUM.
                if idx >= Self::NUM {
                    unsafe { std::hint::unreachable_unchecked() }
                }
                idx
            }
        }
    };
}
