#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub mod avx2;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub use avx2::*;

#[cfg(target_feature = "avx512f")]
pub mod avx512f;

#[cfg(target_feature = "avx512f")]
pub use avx512f::*;

// Preserve interface if we're using a different type.
#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
pub mod simd {
    pub const ARCH_NAME: &str = "fallback";
    pub const CHUNK_SIZE_I16: usize = 1;
    pub const CHUNK_SIZE_F32: usize = 1;
    pub const NB_PACKUS_REGS: usize = 1;
    pub const PACKUS_ORDER: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
}
