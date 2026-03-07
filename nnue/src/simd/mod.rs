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
#[allow(clippy::module_inception)]
pub mod simd {
    pub const ARCH_NAME: &str = "fallback";
    pub const CHUNK_SIZE_I16: usize = 1;
    pub const NB_PACKUS_REGS: usize = 1;
    pub const PACKUS_ORDER: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

    pub type ShiftT = u32;

    pub fn reduce_add(arr: &mut [f32], len: usize) -> f32 {
        debug_assert!(len.is_power_of_two());
        if len == 2 {
            return arr[0] + arr[1];
        }

        for i in 0..(len / 2) {
            arr[i] += arr[i + len / 2];
        }

        reduce_add(arr, len / 2)
    }
}
