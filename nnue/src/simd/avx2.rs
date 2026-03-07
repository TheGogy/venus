pub mod simd {
    use std::arch::x86_64::*;
    pub type IVec = __m256i;
    pub type FVec = __m256;
    pub type Mask16 = __mmask16;
    pub type Mask32 = __mmask32;

    pub const ARCH_NAME: &str = "avx2";

    pub const CHUNK_SIZE_U8: usize = std::mem::size_of::<IVec>() / std::mem::size_of::<u8>();
    pub const CHUNK_SIZE_I16: usize = std::mem::size_of::<IVec>() / std::mem::size_of::<i16>();
    pub const CHUNK_SIZE_I32: usize = std::mem::size_of::<IVec>() / std::mem::size_of::<i32>();
    pub const CHUNK_SIZE_F32: usize = std::mem::size_of::<FVec>() / std::mem::size_of::<f32>();
    pub const NB_PACKUS_REGS: usize = std::mem::size_of::<IVec>() / 8;

    // | 0  2 | 4  6 |
    // | 1  3 | 5  7 |
    pub const PACKUS_ORDER: [usize; 4] = [0, 2, 1, 3];

    /// Returns a zeroed out vector.
    pub fn zeroed_i() -> IVec {
        unsafe { _mm256_setzero_si256() }
    }

    pub fn zeroed_f() -> FVec {
        unsafe { _mm256_setzero_ps() }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_i16(val: i16) -> IVec {
        unsafe { _mm256_set1_epi16(val) }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_i32(val: i32) -> IVec {
        unsafe { _mm256_set1_epi32(val) }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_f32(val: f32) -> FVec {
        unsafe { _mm256_set1_ps(val) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i8(ptr: *const i8) -> IVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm256_load_si256(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i16(ptr: *const i16) -> IVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm256_load_si256(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i32(ptr: *const i32) -> IVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm256_load_si256(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_f32(ptr: *const f32) -> FVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<FVec>()));
        unsafe { _mm256_load_ps(ptr.cast()) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_u8(dst: *mut u8, data: IVec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm256_store_si256(dst.cast(), data) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_i32(dst: *mut i32, data: IVec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm256_store_si256(dst.cast(), data) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_f32(dst: *mut f32, data: FVec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm256_store_ps(dst.cast(), data) }
    }

    /// Multiplies two vectors together and takes the high 16 bits.
    pub fn mulhi_i16(x: IVec, y: IVec) -> IVec {
        unsafe { _mm256_mulhi_epi16(x, y) }
    }

    /// Multiplies two vectors together.
    pub fn mul_f32(x: FVec, y: FVec) -> FVec {
        unsafe { _mm256_mul_ps(x, y) }
    }

    /// Multiplies x and y and adds to z.
    pub fn fmadd_f32(x: FVec, y: FVec, z: FVec) -> FVec {
        unsafe { _mm256_fmadd_ps(x, y, z) }
    }

    /// Returns min of two vectors.
    pub fn min_i16(x: IVec, y: IVec) -> IVec {
        unsafe { _mm256_min_epi16(x, y) }
    }

    /// Returns max of two vectors.
    pub fn max_i16(x: IVec, y: IVec) -> IVec {
        unsafe { _mm256_max_epi16(x, y) }
    }

    /// Clamps a vector between two values.
    pub fn clamp_i16(v: IVec, min: IVec, max: IVec) -> IVec {
        min_i16(max, max_i16(v, min))
    }

    /// Clamps a vector between two values.
    pub fn clamp_f32(v: FVec, min: FVec, max: FVec) -> FVec {
        unsafe { _mm256_min_ps(max, _mm256_max_ps(v, min)) }
    }

    /// Shift left by <SHIFT> and pad with 0s.
    /// HACK: Who decided this one is an i32???
    pub type ShiftT = i32;
    pub fn shl_i16<const SHIFT: ShiftT>(v: IVec) -> IVec {
        unsafe { _mm256_slli_epi16(v, SHIFT) }
    }

    /// Convert packed i16s to u8s with unsigned saturation (0..255).
    pub fn packus_i16_u8(x: IVec, y: IVec) -> IVec {
        unsafe { _mm256_packus_epi16(x, y) }
    }

    /// Convert packed i32s -> f32s.
    pub fn cvt_i32_f32(x: IVec) -> FVec {
        unsafe { _mm256_cvtepi32_ps(x) }
    }

    /// Gets the sum of the values in the vector.
    pub fn reduce_add_f32(v: FVec) -> f32 {
        unsafe {
            let hi = _mm256_extractf128_ps(v, 1);
            let lo = _mm256_castps256_ps128(v);
            let sum_128 = _mm_add_ps(hi, lo);

            let upper_64 = _mm_movehl_ps(sum_128, sum_128);
            let sum_64 = _mm_add_ps(upper_64, sum_128);

            let upper_32 = _mm_shuffle_ps(sum_64, sum_64, 1);
            let sum_32 = _mm_add_ss(upper_32, sum_64);

            _mm_cvtss_f32(sum_32)
        }
    }

    /// Gets a mask of all the nonzero elements in the vector.
    pub fn nonzero_mask_i32(v: IVec) -> Mask16 {
        unsafe { _mm256_movemask_ps(_mm256_castsi256_ps(_mm256_cmpgt_epi32(v, zeroed_i()))) as Mask16 }
    }

    /// Multiply groups of u8s -> i16s -> i32s and sum these with `sum`.
    pub fn dpbusd_i32(sum: IVec, x: IVec, y: IVec) -> IVec {
        unsafe { _mm256_dpbusd_epi32(sum, x, y) }
    }
}
