#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub mod avx2;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub use avx2::*;

#[cfg(target_feature = "avx512f")]
pub mod avx512f;

#[cfg(target_feature = "avx512f")]
pub use avx512f::*;

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
pub mod update_simd;

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
pub use update_simd::*;

#[cfg(all(not(target_feature = "avx2"), not(target_feature = "avx512f")))]
pub mod update_fallback;

#[cfg(all(not(target_feature = "avx2"), not(target_feature = "avx512f")))]
pub use update_fallback::*;
