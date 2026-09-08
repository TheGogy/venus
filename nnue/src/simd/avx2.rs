#[allow(clippy::wildcard_imports)]
use std::arch::x86_64::*;

pub type I8Vec = __m256i;
pub type U8Vec = __m256i;
pub type I16Vec = __m256i;
pub type I32Vec = __m256i;
pub type Mask32 = __mmask32;

pub const ARCH_NAME: &str = "avx2";

pub const U8_LANES: usize = size_of::<U8Vec>() / size_of::<u8>();
pub const I16_LANES: usize = size_of::<I16Vec>() / size_of::<i16>();
pub const I32_LANES: usize = size_of::<I32Vec>() / size_of::<i32>();
pub const PACKUS_REGS: usize = size_of::<I32Vec>() / 8;

// | 0  2 | 4  6 |
// | 1  3 | 5  7 |
pub const PACKUS_ORDER: [usize; 4] = [0, 2, 1, 3];

/// Returns a vector set to the given value.
pub fn splat_i16(val: i16) -> I16Vec {
    unsafe { _mm256_set1_epi16(val) }
}

/// Returns a vector set to the given value.
pub fn splat_i32(val: i32) -> I32Vec {
    unsafe { _mm256_set1_epi32(val) }
}

/// Loads a vector in directly from the values at the given pointer.
///
/// # Safety
/// `ptr` must be valid for a read of one vector, and aligned to it.
pub unsafe fn load_i8(ptr: *const i8) -> I8Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I8Vec>()));
    unsafe { _mm256_load_si256(ptr.cast()) }
}

/// Loads a vector in directly from the values at the given pointer.
///
/// # Safety
/// `ptr` must be valid for a read of one vector, and aligned to it.
pub unsafe fn load_i16(ptr: *const i16) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { _mm256_load_si256(ptr.cast()) }
}

/// Loads [`I16_LANES`] i8s and sign extends them into a vector of i16s.
///
/// # Safety
/// `ptr` must be valid for a read of [`I16_LANES`] bytes, and aligned to that many bytes.
pub unsafe fn load_extend_i8(ptr: *const i8) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(I16_LANES));
    unsafe { _mm256_cvtepi8_epi16(_mm_load_si128(ptr.cast())) }
}

/// Loads a vector in directly from the values at the given pointer.
///
/// # Safety
/// `ptr` must be valid for a read of one vector, and aligned to it.
pub unsafe fn load_i32(ptr: *const i32) -> I32Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { _mm256_load_si256(ptr.cast()) }
}

/// Stores a vector at the given pointer.
///
/// # Safety
/// `dst` must be valid for a write of one vector, and aligned to it.
pub unsafe fn store_u8(dst: *mut u8, data: U8Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<U8Vec>()));
    unsafe { _mm256_store_si256(dst.cast(), data) }
}

/// Stores a vector at the given pointer.
///
/// # Safety
/// `dst` must be valid for a write of one vector, and aligned to it.
pub unsafe fn store_i16(dst: *mut i16, data: I16Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { _mm256_store_si256(dst.cast(), data) }
}

/// Stores a vector at the given pointer.
///
/// # Safety
/// `dst` must be valid for a write of one vector, and aligned to it.
pub unsafe fn store_i32(dst: *mut i32, data: I32Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { _mm256_store_si256(dst.cast(), data) }
}

/// Multiplies two vectors together, keeping the rounded high 16 bits of `2 * x * y`.
pub fn mulhrs_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm256_mulhrs_epi16(x, y) }
}

/// Sums two vectors together.
pub fn add_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm256_add_epi16(x, y) }
}

/// Subtracts the second vector from the first.
pub fn sub_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm256_sub_epi16(x, y) }
}

/// Returns min of two vectors.
pub fn min_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm256_min_epi16(x, y) }
}

/// Returns max of two vectors.
pub fn max_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm256_max_epi16(x, y) }
}

/// Clamps a vector between two values.
pub fn clamp_i16(v: I16Vec, min: I16Vec, max: I16Vec) -> I16Vec {
    min_i16(max, max_i16(v, min))
}

/// Shift left by <SHIFT> and pad with 0s.
/// HACK: Who decided this one is an i32???
pub type ShiftT = i32;
pub fn shl_i16<const SHIFT: ShiftT>(v: I16Vec) -> I16Vec {
    unsafe { _mm256_slli_epi16(v, SHIFT) }
}

/// Convert packed i16s to u8s with unsigned saturation (0..255).
pub fn packus_i16_u8(x: I16Vec, y: I16Vec) -> U8Vec {
    unsafe { _mm256_packus_epi16(x, y) }
}

/// Sums two vectors together.
pub fn add_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { _mm256_add_epi32(x, y) }
}

/// Multiplies two vectors together, keeping the low 32 bits of each product.
pub fn mul_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { _mm256_mullo_epi32(x, y) }
}

/// Multiplies x and y and adds to z.
pub fn mul_add_i32(x: I32Vec, y: I32Vec, z: I32Vec) -> I32Vec {
    add_i32(mul_i32(x, y), z)
}

/// Returns min of two vectors.
pub fn min_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { _mm256_min_epi32(x, y) }
}

/// Returns max of two vectors.
pub fn max_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { _mm256_max_epi32(x, y) }
}

/// Clamps a vector between two values.
pub fn clamp_i32(v: I32Vec, min: I32Vec, max: I32Vec) -> I32Vec {
    min_i32(max, max_i32(v, min))
}

/// Shift left by <SHIFT> and pad with 0s.
pub fn shl_i32<const SHIFT: ShiftT>(v: I32Vec) -> I32Vec {
    unsafe { _mm256_slli_epi32(v, SHIFT) }
}

/// Arithmetic shift right by <SHIFT>.
pub fn shr_i32<const SHIFT: ShiftT>(v: I32Vec) -> I32Vec {
    unsafe { _mm256_srai_epi32(v, SHIFT) }
}

/// Gets the sum of the values in the vector.
pub fn reduce_add_i32(v: I32Vec) -> i32 {
    unsafe {
        let hi = _mm256_extracti128_si256::<1>(v);
        let lo = _mm256_castsi256_si128(v);
        let sum_128 = _mm_add_epi32(hi, lo);

        let sum_64 = _mm_add_epi32(sum_128, _mm_shuffle_epi32::<0b0100_1110>(sum_128));
        let sum_32 = _mm_add_epi32(sum_64, _mm_shuffle_epi32::<0b1011_0001>(sum_64));

        _mm_cvtsi128_si32(sum_32)
    }
}

/// Gets a mask of all the nonzero elements in the vector.
/// Only the low [`I32_LANES`] bits are ever set, so the sign bit is never involved.
#[allow(clippy::cast_sign_loss)]
pub fn nonzero_mask_i32(v: I32Vec) -> Mask32 {
    unsafe { _mm256_movemask_ps(_mm256_castsi256_ps(_mm256_cmpgt_epi32(v, _mm256_setzero_si256()))) as Mask32 }
}

/// Multiply groups of u8s -> i16s -> i32s and sum these with `sum`.
pub fn dotprod_i32(sum: I32Vec, x: U8Vec, y: I8Vec) -> I32Vec {
    unsafe { _mm256_dpbusd_epi32(sum, x, y) }
}

/// Convert packed u8s -> i32s.
/// No-op for x86 arch.
pub const fn cast_u8_i32(x: U8Vec) -> I32Vec {
    x
}

/// Convert packed i32s -> u8s.
/// No-op for x86 arch.
pub const fn cast_i32_u8(x: I32Vec) -> U8Vec {
    x
}

/// Fetch the cache line at `ptr` into every level of cache.
pub fn prefetch(ptr: *const u8) {
    unsafe { _mm_prefetch::<_MM_HINT_T0>(ptr.cast()) }
}
