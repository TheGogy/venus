#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub mod avx2;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub use avx2::*;

#[cfg(target_feature = "avx512f")]
pub mod avx512f;

#[cfg(target_feature = "avx512f")]
pub use avx512f::*;
