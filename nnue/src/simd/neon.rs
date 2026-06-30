pub mod simd {
    #[allow(clippy::wildcard_imports)]
    use std::arch::aarch64::*;

    pub type I8Vec = int8x16_t;
    pub type U8Vec = uint8x16_t;
    pub type I16Vec = int16x8_t;
    pub type I32Vec = int32x4_t;
    pub type U32Vec = uint32x4_t;
    pub type F32Vec = float32x4_t;
    pub type Mask32 = u32;

    pub const ARCH_NAME: &str = "neon";

    pub const CHUNK_SIZE_U8: usize = std::mem::size_of::<U8Vec>() / std::mem::size_of::<u8>();
    pub const CHUNK_SIZE_I16: usize = std::mem::size_of::<I16Vec>() / std::mem::size_of::<i16>();
    pub const CHUNK_SIZE_I32: usize = std::mem::size_of::<I32Vec>() / std::mem::size_of::<i32>();
    pub const CHUNK_SIZE_F32: usize = std::mem::size_of::<F32Vec>() / std::mem::size_of::<f32>();
    pub const NB_PACKUS_REGS: usize = std::mem::size_of::<I32Vec>() / 8;

    // | 0  1 |
    pub const PACKUS_ORDER: [usize; 2] = [0, 1];

    /// Returns a vector set to the given value.
    pub fn from_val_i16(val: i16) -> I16Vec {
        unsafe { vdupq_n_s16(val) }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_i32(val: i32) -> I32Vec {
        unsafe { vdupq_n_s32(val) }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_f32(val: f32) -> F32Vec {
        unsafe { vdupq_n_f32(val) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i8(ptr: *const i8) -> I8Vec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<I8Vec>()));
        unsafe { vld1q_s8(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i16(ptr: *const i16) -> I16Vec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<I16Vec>()));
        unsafe { vld1q_s16(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i32(ptr: *const i32) -> I32Vec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<I32Vec>()));
        unsafe { vld1q_s32(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_f32(ptr: *const f32) -> F32Vec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<F32Vec>()));
        unsafe { vld1q_f32(ptr.cast()) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_u8(dst: *mut u8, data: U8Vec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<U8Vec>()));
        unsafe { vst1q_u8(dst.cast(), data) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_i32(dst: *mut i32, data: I32Vec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<I32Vec>()));
        unsafe { vst1q_s32(dst.cast(), data) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_f32(dst: *mut f32, data: F32Vec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<F32Vec>()));
        unsafe { vst1q_f32(dst.cast(), data) }
    }

    /// Multiplies two vectors together and takes the high 16 bits.
    /// WARN: This is a DOUBLED mulhi!!
    pub fn mulhi_i16(x: I16Vec, y: I16Vec) -> I16Vec {
        unsafe { vqdmulhq_s16(x, y) }
    }

    /// Multiplies two vectors together.
    pub fn mul_f32(x: F32Vec, y: F32Vec) -> F32Vec {
        unsafe { vmulq_f32(x, y) }
    }

    /// Sums two vectors together.
    pub fn add_f32(x: F32Vec, y: F32Vec) -> F32Vec {
        unsafe { vaddq_f32(x, y) }
    }

    /// Multiplies x and y and adds to z.
    pub fn fmadd_f32(x: F32Vec, y: F32Vec, z: F32Vec) -> F32Vec {
        unsafe { vfmaq_f32(z, x, y) }
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

    /// Clamps a vector between two values.
    pub fn clamp_f32(v: F32Vec, min: F32Vec, max: F32Vec) -> F32Vec {
        unsafe { vminq_f32(max, vmaxq_f32(v, min)) }
    }

    /// Retuns min of two values.
    pub fn min_f32(x: F32Vec, y: F32Vec) -> F32Vec {
        unsafe { vminq_f32(x, y) }
    }

    /// Shift left by <SHIFT> and pad with 0s.
    /// HACK: Have to accommodate for avx2. Who wrote this interface
    pub type ShiftT = i32;
    pub fn shl_i16<const SHIFT: ShiftT>(v: I16Vec) -> I16Vec {
        unsafe { vshlq_n_s16::<SHIFT>(v) }
    }

    /// Convert packed i16s to u8s with unsigned saturation (0..255).
    pub fn packus_i16_u8(x: I16Vec, y: I16Vec) -> U8Vec {
        unsafe { vcombine_u8(vqmovun_s16(x), vqmovun_s16(y)) }
    }

    /// Convert packed i32s -> f32s.
    pub fn cvt_i32_f32(x: I32Vec) -> F32Vec {
        unsafe { vcvtq_f32_s32(x) }
    }

    /// Gets the sum of the values in the vector.
    pub fn reduce_add_f32(v: F32Vec) -> f32 {
        unsafe { vaddvq_f32(v) }
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
    pub fn reinterpret_i32_u8(x: I32Vec) -> U8Vec {
        unsafe { std::mem::transmute(x) }
    }

    /// Reinterpret packed i32s -> u8s.
    pub fn reinterpret_u8_i32(x: U8Vec) -> I32Vec {
        unsafe { std::mem::transmute(x) }
    }
}
