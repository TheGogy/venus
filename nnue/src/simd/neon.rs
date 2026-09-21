#[allow(clippy::wildcard_imports)]
use std::arch::aarch64::*;

pub type I8Vec = int8x16_t;
pub type U8Vec = uint8x16_t;
pub type I16Vec = int16x8_t;
pub type I32Vec = int32x4_t;
pub type F32Vec = float32x4_t;
pub type Mask32 = u32;
pub type ShiftT = i32;

pub const ARCH_NAME: &str = "neon";

pub const U8_LANES: usize = size_of::<U8Vec>() / size_of::<u8>();
pub const I16_LANES: usize = size_of::<I16Vec>() / size_of::<i16>();
pub const I32_LANES: usize = size_of::<I32Vec>() / size_of::<i32>();
pub const F32_LANES: usize = size_of::<F32Vec>() / size_of::<f32>();
pub const PACKUS_REGS: usize = size_of::<I32Vec>() / 8;

// | 0  1 |
pub const PACKUS_ORDER: [usize; 2] = [0, 1];

pub fn splat_i16(val: i16) -> I16Vec {
    unsafe { vdupq_n_s16(val) }
}

pub fn splat_i32(val: i32) -> I32Vec {
    unsafe { vdupq_n_s32(val) }
}

pub fn splat_f32(val: f32) -> F32Vec {
    unsafe { vdupq_n_f32(val) }
}

pub unsafe fn load_i8(ptr: *const i8) -> I8Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I8Vec>()));
    unsafe { vld1q_s8(ptr) }
}

pub unsafe fn load_i16(ptr: *const i16) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { vld1q_s16(ptr) }
}

pub unsafe fn load_extend_i8(ptr: *const i8) -> I16Vec {
    debug_assert!((ptr as usize).is_multiple_of(I16_LANES));
    unsafe { vmovl_s8(vld1_s8(ptr)) }
}

pub unsafe fn load_i32(ptr: *const i32) -> I32Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { vld1q_s32(ptr) }
}

pub unsafe fn load_f32(ptr: *const f32) -> F32Vec {
    debug_assert!((ptr as usize).is_multiple_of(align_of::<F32Vec>()));
    unsafe { vld1q_f32(ptr) }
}

pub unsafe fn store_u8(dst: *mut u8, data: U8Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<U8Vec>()));
    unsafe { vst1q_u8(dst, data) }
}

pub unsafe fn store_i16(dst: *mut i16, data: I16Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I16Vec>()));
    unsafe { vst1q_s16(dst, data) }
}

pub unsafe fn store_i32(dst: *mut i32, data: I32Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<I32Vec>()));
    unsafe { vst1q_s32(dst, data) }
}

pub unsafe fn store_f32(dst: *mut f32, data: F32Vec) {
    debug_assert!((dst as usize).is_multiple_of(align_of::<F32Vec>()));
    unsafe { vst1q_f32(dst, data) }
}

pub fn add_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vaddq_s16(x, y) }
}

pub fn sub_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vsubq_s16(x, y) }
}

pub fn clamp_i16(v: I16Vec, min: I16Vec, max: I16Vec) -> I16Vec {
    unsafe { vminq_s16(max, vmaxq_s16(v, min)) }
}

/// Multiplies two vectors together and shifts the whole product right by `SHIFT`.
pub fn mulshr_u16<const SHIFT: ShiftT>(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vreinterpretq_s16_u16(vshrq_n_u16::<SHIFT>(vreinterpretq_u16_s16(vmulq_s16(x, y)))) }
}

/// Multiplies two vectors together, keeping the low 16 bits of each product.
pub fn mul_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    unsafe { vmulq_s16(x, y) }
}

/// Multiplies two vectors together and rounds each product to its top half, `(x * y + 2^14) >> 15`.
pub fn mulhrs_i16(x: I16Vec, y: I16Vec) -> I16Vec {
    // NOTE: Saturates at -32768 * -32768.
    unsafe { vqrdmulhq_s16(x, y) }
}

/// Convert packed i16s to u8s with unsigned saturation (0..255), in [`PACKUS_ORDER`].
pub fn packus_i16_u8(x: I16Vec, y: I16Vec) -> U8Vec {
    unsafe { vcombine_u8(vqmovun_s16(x), vqmovun_s16(y)) }
}

/// Multiply groups of 4 u8s by 4 i8s and accumulate each group into one lane of `sum`.
pub fn dotprod_i32(sum: I32Vec, x: U8Vec, y: I8Vec) -> I32Vec {
    // Need i8mm for unsigned/signed dotprod.
    #[cfg(target_feature = "i8mm")]
    unsafe {
        // vusdotq_s32(sum, x, y)
        let ret: I32Vec;
        std::arch::asm!(
            "usdot {acc:v}.4s, {x:v}.16b, {y:v}.16b",
            acc = inout(vreg) sum => ret,
            x = in(vreg) x,
            y = in(vreg) y,
            options(pure, nomem, nostack, preserves_flags)
        );
        ret
    }

    // Emulate the unsigned x signed product: bias `x` down by 128
    // (x ^ 0x80 == x - 128) before the signed dot product, then add back
    // `128 * sum4(y)` per group of 4 to correct for the bias:
    // sdot(x - 128, y) = sum(x*y) - 128 * sum(y)
    #[cfg(all(target_feature = "dotprod", not(target_feature = "i8mm")))]
    unsafe {
        let x_biased = vreinterpretq_s8_u8(veorq_u8(x, vdupq_n_u8(0x80)));
        let mut ret: I32Vec;
        std::arch::asm!(
            "sdot {acc:v}.4s, {x:v}.16b, {y:v}.16b",
            acc = inout(vreg) sum => ret,
            x = in(vreg) x_biased,
            y = in(vreg) y,
            options(pure, nomem, nostack, preserves_flags)
        );
        let zero = vdupq_n_s32(0);
        let y_sum4: I32Vec;
        std::arch::asm!(
            "sdot {acc:v}.4s, {y:v}.16b, {ones:v}.16b",
            acc = inout(vreg) zero => y_sum4,
            y = in(vreg) y,
            ones = in(vreg) vdupq_n_s8(1),
            options(pure, nomem, nostack, preserves_flags)
        );
        ret = vaddq_s32(ret, vshlq_n_s32::<7>(y_sum4));
        ret
    }

    // All other systems get full fallback.
    #[cfg(not(any(target_feature = "dotprod", target_feature = "i8mm")))]
    unsafe {
        let x = (vreinterpretq_s16_u16(vmovl_u8(vget_low_u8(x))), vreinterpretq_s16_u16(vmovl_high_u8(x)));
        let y = (vmovl_s8(vget_low_s8(y)), vmovl_high_s8(y));
        let lo = vpaddlq_s16(vmulq_s16(x.0, y.0));
        let hi = vpaddlq_s16(vmulq_s16(x.1, y.1));
        vaddq_s32(sum, vpaddq_s32(lo, hi))
    }
}

pub fn cvt_i32_f32(x: I32Vec) -> F32Vec {
    unsafe { vcvtq_f32_s32(x) }
}

pub fn add_f32(x: F32Vec, y: F32Vec) -> F32Vec {
    unsafe { vaddq_f32(x, y) }
}

pub fn mul_f32(x: F32Vec, y: F32Vec) -> F32Vec {
    unsafe { vmulq_f32(x, y) }
}

pub fn fmadd_f32(x: F32Vec, y: F32Vec, z: F32Vec) -> F32Vec {
    unsafe { vfmaq_f32(z, x, y) }
}

pub fn min_f32(x: F32Vec, y: F32Vec) -> F32Vec {
    unsafe { vminq_f32(x, y) }
}

pub fn clamp_f32(v: F32Vec, min: F32Vec, max: F32Vec) -> F32Vec {
    unsafe { vminq_f32(max, vmaxq_f32(v, min)) }
}

pub fn reduce_add_f32(v: F32Vec) -> f32 {
    unsafe { vaddv_f32(vadd_f32(vget_low_f32(v), vget_high_f32(v))) }
}

pub fn nonzero_mask_i32(v: I32Vec) -> Mask32 {
    const LANE_BITS: [u32; 4] = [1, 2, 4, 8];
    unsafe {
        let v = vreinterpretq_u32_s32(v);
        vaddvq_u32(vandq_u32(vtstq_u32(v, v), vld1q_u32(LANE_BITS.as_ptr())))
    }
}

pub fn cast_u8_i32(x: U8Vec) -> I32Vec {
    unsafe { vreinterpretq_s32_u8(x) }
}

pub fn cast_i32_u8(x: I32Vec) -> U8Vec {
    unsafe { vreinterpretq_u8_s32(x) }
}

#[allow(unused_variables, clippy::missing_const_for_fn)]
pub fn prefetch(ptr: *const u8) {}
