#[cfg(target_feature = "avx512f")]
pub mod avx512f;
#[cfg(target_feature = "avx512f")]
pub use avx512f::*;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub mod avx2;
#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub use avx2::*;

#[cfg(target_feature = "neon")]
pub mod neon;
#[cfg(target_feature = "neon")]
pub use neon::*;

#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon")))]
compile_error!("No SIMD backend available: build with target-cpu=native, or enable one of avx2, avx512f or neon.");
