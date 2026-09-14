#[allow(clippy::wildcard_imports)]
use std::arch::x86_64::*;

pub type I8Vec = __m512i;
pub type U8Vec = __m512i;
pub type I16Vec = __m512i;
pub type I32Vec = __m512i;
pub type F32Vec = __m512;
pub type Mask32 = u32;
pub type ShiftT = u32;

pub const ARCH_NAME: &str = "avx512";

pub const U8_LANES: usize = size_of::<U8Vec>() / size_of::<u8>();
pub const I16_LANES: usize = size_of::<I16Vec>() / size_of::<i16>();
pub const I32_LANES: usize = size_of::<I32Vec>() / size_of::<i32>();
pub const F32_LANES: usize = size_of::<F32Vec>() / size_of::<f32>();
pub const PACKUS_REGS: usize = size_of::<I32Vec>() / 8;

// | 0  2  4  6 |
// | 1  3  5  7 |
pub const PACKUS_ORDER: [usize; 8] = [0, 2, 4, 6, 1, 3, 5, 7];

pub fn splat_i16(val: i16) -> I16Vec {
    unsafe { _mm512_set1_epi16(val) }
}

pub fn splat_i32(val: i32) -> I32Vec {
    unsafe { _mm512_set1_epi32(val) }
}

pub fn splat_f32(val: f32) -> F32Vec {
    unsafe { _mm512_set1_ps(val) }
}

pub unsafe fn load_i8(ptr: *const i8) -> I8Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I8Vec>()));
    unsafe { _mm512_load_si512(ptr.cast()) }
}

pub unsafe fn load_i16(ptr: *const i16) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { _mm512_load_si512(ptr.cast()) }
}

pub unsafe fn load_extend_i8(ptr: *const i8) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(I16_LANES));
    unsafe { _mm512_cvtepi8_epi16(_mm256_load_si256(ptr.cast())) }
}

pub unsafe fn load_i32(ptr: *const i32) -> I32Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { _mm512_load_si512(ptr.cast()) }
}

pub unsafe fn load_f32(ptr: *const f32) -> F32Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<F32Vec>()));
    unsafe { _mm512_load_ps(ptr.cast()) }
}

pub unsafe fn store_u8(dst: *mut u8, data: U8Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<U8Vec>()));
    unsafe { _mm512_store_si512(dst.cast(), data) }
}

pub unsafe fn store_i16(dst: *mut i16, data: I16Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { _mm512_store_si512(dst.cast(), data) }
}

pub unsafe fn store_i32(dst: *mut i32, data: I32Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { _mm512_store_si512(dst.cast(), data) }
}

pub unsafe fn store_f32(dst: *mut f32, data: F32Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<F32Vec>()));
    unsafe { _mm512_store_ps(dst.cast(), data) }
}

pub fn add_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm512_add_epi16(x, y) }
}

pub fn sub_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm512_sub_epi16(x, y) }
}

pub fn clamp_i16(v: I16Vec, min: I16Vec, max: I16Vec) -> I16Vec {
    unsafe { _mm512_min_epi16(max, _mm512_max_epi16(v, min)) }
}

/// Multiplies two vectors together and shifts the whole product right by `SHIFT`.
pub fn mulshr_u16<const SHIFT: ShiftT>(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { _mm512_srli_epi16(_mm512_mullo_epi16(x, y), SHIFT) }
}

/// Convert packed i16s to u8s with unsigned saturation (0..255), in [`PACKUS_ORDER`].
pub fn packus_i16_u8(x: I16Vec, y: I16Vec) -> U8Vec {
    unsafe { _mm512_packus_epi16(x, y) }
}

/// Multiply groups of 4 u8s by 4 i8s and accumulate each group into one lane of `sum`.
pub fn dotprod_i32(sum: I32Vec, x: U8Vec, y: I8Vec) -> I32Vec {
    #[cfg(target_feature = "avx512vnni")]
    unsafe {
        _mm512_dpbusd_epi32(sum, x, y)
    }

    // Have to avoid i16 intermediate overflow.
    #[cfg(not(target_feature = "avx512vnni"))]
    unsafe {
        let ones = _mm512_set1_epi16(1);
        let hi = _mm512_set1_epi8(0x80u8 as i8);
        let s1 = _mm512_maddubs_epi16(_mm512_and_si512(x, hi), y);
        let s2 = _mm512_maddubs_epi16(_mm512_andnot_si512(hi, x), y);
        let s1 = _mm512_madd_epi16(s1, ones);
        let s2 = _mm512_madd_epi16(s2, ones);
        _mm512_add_epi32(sum, _mm512_add_epi32(s1, s2))
    }
}

pub fn cvt_i32_f32(x: I32Vec) -> F32Vec {
    unsafe { _mm512_cvtepi32_ps(x) }
}

pub fn add_f32(x: F32Vec, y: F32Vec) -> F32Vec {
    unsafe { _mm512_add_ps(x, y) }
}

pub fn mul_f32(x: F32Vec, y: F32Vec) -> F32Vec {
    unsafe { _mm512_mul_ps(x, y) }
}

pub fn fmadd_f32(x: F32Vec, y: F32Vec, z: F32Vec) -> F32Vec {
    unsafe { _mm512_fmadd_ps(x, y, z) }
}

pub fn min_f32(x: F32Vec, y: F32Vec) -> F32Vec {
    unsafe { _mm512_min_ps(x, y) }
}

pub fn clamp_f32(v: F32Vec, min: F32Vec, max: F32Vec) -> F32Vec {
    unsafe { _mm512_min_ps(max, _mm512_max_ps(v, min)) }
}

pub fn reduce_add_f32(v: F32Vec) -> f32 {
    unsafe { _mm512_reduce_add_ps(v) }
}

pub fn nonzero_mask_i32(v: I32Vec) -> Mask32 {
    unsafe { Mask32::from(_mm512_test_epi32_mask(v, v)) }
}

/// No-op: x86 keeps u8s and i32s in the same registers.
pub const fn cast_u8_i32(x: U8Vec) -> I32Vec {
    x
}

/// No-op: x86 keeps u8s and i32s in the same registers.
pub const fn cast_i32_u8(x: I32Vec) -> U8Vec {
    x
}

/// Fetch the cache line at `ptr` into every level of cache.
pub fn prefetch(ptr: *const u8) {
    unsafe { _mm_prefetch::<_MM_HINT_T0>(ptr.cast()) }
}
