#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
#[path = "avx512.rs"]
mod backend;

#[cfg(all(target_feature = "avx2", not(all(target_feature = "avx512f", target_feature = "avx512bw"))))]
#[path = "avx2.rs"]
mod backend;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
#[path = "neon.rs"]
mod backend;

#[cfg(not(any(target_feature = "avx512f", target_feature = "avx2", target_feature = "neon")))]
compile_error!("No SIMD backend available: build with target-cpu=native, or enable one of avx2, avx512f+avx512bw or neon.");

pub use backend::*;
