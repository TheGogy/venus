#[allow(clippy::wildcard_imports)]
use std::arch::aarch64::*;

pub type I8Vec = int8x16_t;
pub type U8Vec = uint8x16_t;
pub type I16Vec = int16x8_t;
pub type I32Vec = int32x4_t;
pub type U32Vec = uint32x4_t;
pub type Mask32 = u32;

pub const ARCH_NAME: &str = "neon";

pub const U8_LANES: usize = size_of::<U8Vec>() / size_of::<u8>();
pub const I16_LANES: usize = size_of::<I16Vec>() / size_of::<i16>();
pub const I32_LANES: usize = size_of::<I32Vec>() / size_of::<i32>();
pub const PACKUS_REGS: usize = size_of::<I32Vec>() / 8;

// | 0  1 |
pub const PACKUS_ORDER: [usize; 2] = [0, 1];

/// Returns a vector set to the given value.
pub fn splat_i16(val: i16) -> I16Vec {
    unsafe { vdupq_n_s16(val) }
}

/// Returns a vector set to the given value.
pub fn splat_i32(val: i32) -> I32Vec {
    unsafe { vdupq_n_s32(val) }
}

/// Loads a vector in directly from the values at the given pointer.
///
/// # Safety
/// `ptr` must be valid for a read of one vector, and aligned to it.
pub unsafe fn load_i8(ptr: *const i8) -> I8Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I8Vec>()));
    unsafe { vld1q_s8(ptr.cast()) }
}

/// Loads a vector in directly from the values at the given pointer.
///
/// # Safety
/// `ptr` must be valid for a read of one vector, and aligned to it.
pub unsafe fn load_i16(ptr: *const i16) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { vld1q_s16(ptr.cast()) }
}

/// Loads [`I16_LANES`] i8s and sign extends them into a vector of i16s.
///
/// # Safety
/// `ptr` must be valid for a read of [`I16_LANES`] bytes, and aligned to that many bytes.
pub unsafe fn load_extend_i8(ptr: *const i8) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(I16_LANES));
    unsafe { vmovl_s8(vld1_s8(ptr.cast())) }
}

/// Loads a vector in directly from the values at the given pointer.
///
/// # Safety
/// `ptr` must be valid for a read of one vector, and aligned to it.
pub unsafe fn load_i32(ptr: *const i32) -> I32Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { vld1q_s32(ptr.cast()) }
}

/// Stores a vector at the given pointer.
///
/// # Safety
/// `dst` must be valid for a write of one vector, and aligned to it.
pub unsafe fn store_u8(dst: *mut u8, data: U8Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<U8Vec>()));
    unsafe { vst1q_u8(dst.cast(), data) }
}

/// Stores a vector at the given pointer.
///
/// # Safety
/// `dst` must be valid for a write of one vector, and aligned to it.
pub unsafe fn store_i16(dst: *mut i16, data: I16Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { vst1q_s16(dst.cast(), data) }
}

/// Stores a vector at the given pointer.
///
/// # Safety
/// `dst` must be valid for a write of one vector, and aligned to it.
pub unsafe fn store_i32(dst: *mut i32, data: I32Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { vst1q_s32(dst.cast(), data) }
}

/// Multiplies two vectors together, keeping the rounded high 16 bits of `2 * x * y`.
pub fn mulhrs_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vqrdmulhq_s16(x, y) }
}

/// Sums two vectors together.
pub fn add_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vaddq_s16(x, y) }
}

/// Subtracts the second vector from the first.
pub fn sub_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vsubq_s16(x, y) }
}

/// Returns min of two vectors.
pub fn min_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vminq_s16(x, y) }
}

/// Returns max of two vectors.
pub fn max_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vmaxq_s16(x, y) }
}

/// Clamps a vector between two values.
pub fn clamp_i16(v: I16Vec, min: I16Vec, max: I16Vec) -> I16Vec {
    min_i16(max, max_i16(v, min))
}

/// Shift left by <SHIFT> and pad with 0s.
/// HACK: Have to accommodate for avx2.
pub type ShiftT = i32;
pub fn shl_i16<const SHIFT: ShiftT>(v: I16Vec) -> I16Vec {
    unsafe { vshlq_n_s16::<SHIFT>(v) }
}

/// Convert packed i16s to u8s with unsigned saturation (0..255).
pub fn packus_i16_u8(x: I16Vec, y: I16Vec) -> U8Vec {
    unsafe { vcombine_u8(vqmovun_s16(x), vqmovun_s16(y)) }
}

/// Sums two vectors together.
pub fn add_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { vaddq_s32(x, y) }
}

/// Multiplies two vectors together, keeping the low 32 bits of each product.
pub fn mul_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { vmulq_s32(x, y) }
}

/// Multiplies x and y and adds to z.
pub fn mul_add_i32(x: I32Vec, y: I32Vec, z: I32Vec) -> I32Vec {
    unsafe { vmlaq_s32(z, x, y) }
}

/// Returns min of two vectors.
pub fn min_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { vminq_s32(x, y) }
}

/// Returns max of two vectors.
pub fn max_i32(x: I32Vec, y: I32Vec) -> I32Vec {
    unsafe { vmaxq_s32(x, y) }
}

/// Clamps a vector between two values.
pub fn clamp_i32(v: I32Vec, min: I32Vec, max: I32Vec) -> I32Vec {
    min_i32(max, max_i32(v, min))
}

/// Shift left by <SHIFT> and pad with 0s.
pub fn shl_i32<const SHIFT: ShiftT>(v: I32Vec) -> I32Vec {
    unsafe { vshlq_n_s32::<SHIFT>(v) }
}

/// Arithmetic shift right by <SHIFT>.
pub fn shr_i32<const SHIFT: ShiftT>(v: I32Vec) -> I32Vec {
    unsafe { vshrq_n_s32::<SHIFT>(v) }
}

/// Gets the sum of the values in the vector.
pub fn reduce_add_i32(v: I32Vec) -> i32 {
    unsafe { vaddvq_s32(v) }
}

/// Gets a mask of all the nonzero elements in the vector.
pub fn nonzero_mask_i32(v: I32Vec) -> Mask32 {
    unsafe {
        const MASK: [u32; 4] = [1, 2, 4, 8];
        let vu32: U32Vec = std::mem::transmute(v);
        vaddvq_u32(vandq_u32(vtstq_u32(vu32, vu32), vld1q_u32(MASK.as_ptr())))
    }
}

/// Multiply groups of u8s -> i16s -> i32s and sum these with `sum`.
pub fn dotprod_i32(sum: I32Vec, x: U8Vec, y: I8Vec) -> I32Vec {
    unsafe { vdotq_s32(sum, std::mem::transmute::<U8Vec, I8Vec>(x), y) }
}

/// Reinterpret packed i32s -> u8s.
pub fn cast_i32_u8(x: I32Vec) -> U8Vec {
    unsafe { std::mem::transmute(x) }
}

/// Reinterpret packed i32s -> u8s.
pub fn cast_u8_i32(x: U8Vec) -> I32Vec {
    unsafe { std::mem::transmute(x) }
}

/// Fetch the cache line at `ptr` into every level of cache.
/// The prefetch intrinsics are still unstable on aarch64, so this is a no-op.
#[allow(unused_variables, clippy::missing_const_for_fn)]
pub fn prefetch(ptr: *const u8) {}
