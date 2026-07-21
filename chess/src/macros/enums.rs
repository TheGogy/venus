/// Implements a number of helper functions for a given enum.
///
/// # Example
///
///```
/// impl_from_type! {
///     Square, u8, 64
/// }
/// ```
#[macro_export]
macro_rules! impl_from_type {
    ($t:ty, $inner:ty, $max:expr) => {
        impl $t {
            /// The total number of values for this type.
            pub const NUM: usize = $max;

            /// Gets the index for this type.
            pub const fn idx(self) -> usize {
                assert!(self as usize <= $max);
                self as usize
            }

            /// Constructs the type from the inner primitive.
            /// Safety: caller guarantees this is within bounds.
            pub const fn from_raw(i: $inner) -> Self {
                assert!(i < $max);
                unsafe { std::mem::transmute(i as $inner) }
            }

            pub const fn to_raw(self) -> $inner {
                assert!((self as usize) < $max);
                self as $inner
            }
        }
    };
}
