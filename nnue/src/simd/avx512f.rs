pub mod simd {
    use std::arch::x86_64::{
        __m512, __m512i, __mmask16, __mmask32, _mm512_cmpgt_epi32_mask, _mm512_cvtepi32_ps, _mm512_dpbusd_epi32, _mm512_fmadd_ps,
        _mm512_load_ps, _mm512_load_si512, _mm512_max_epi16, _mm512_max_ps, _mm512_min_epi16, _mm512_min_ps, _mm512_mul_ps,
        _mm512_mulhi_epi16, _mm512_packus_epi16, _mm512_reduce_add_ps, _mm512_set1_epi16, _mm512_set1_epi32, _mm512_set1_ps,
        _mm512_setzero_ps, _mm512_setzero_si512, _mm512_slli_epi16, _mm512_store_ps, _mm512_store_si512,
    };
    pub type IVec = __m512i;
    pub type FVec = __m512;
    pub type Mask16 = __mmask16;
    pub type Mask32 = __mmask32;

    pub const ARCH_NAME: &str = "avx512f";

    pub const CHUNK_SIZE_U8: usize = std::mem::size_of::<IVec>() / std::mem::size_of::<u8>();
    pub const CHUNK_SIZE_I16: usize = std::mem::size_of::<IVec>() / std::mem::size_of::<i16>();
    pub const CHUNK_SIZE_I32: usize = std::mem::size_of::<IVec>() / std::mem::size_of::<i32>();
    pub const CHUNK_SIZE_F32: usize = std::mem::size_of::<FVec>() / std::mem::size_of::<f32>();
    pub const NB_PACKUS_REGS: usize = std::mem::size_of::<IVec>() / 8;

    // | 0  2  4  6 |
    // | 1  3  5  7 |
    pub const PACKUS_ORDER: [usize; 8] = [0, 2, 4, 6, 1, 3, 5, 7];

    /// Returns a zeroed out vector.
    pub fn zeroed_i() -> IVec {
        unsafe { _mm512_setzero_si512() }
    }

    pub fn zeroed_f() -> FVec {
        unsafe { _mm512_setzero_ps() }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_i16(val: i16) -> IVec {
        unsafe { _mm512_set1_epi16(val) }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_i32(val: i32) -> IVec {
        unsafe { _mm512_set1_epi32(val) }
    }

    /// Returns a vector set to the given value.
    pub fn from_val_f32(val: f32) -> FVec {
        unsafe { _mm512_set1_ps(val) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i8(ptr: *const i8) -> IVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm512_load_si512(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i16(ptr: *const i16) -> IVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm512_load_si512(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_i32(ptr: *const i32) -> IVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm512_load_si512(ptr.cast()) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr_f32(ptr: *const f32) -> FVec {
        debug_assert!((ptr as usize).is_multiple_of(std::mem::align_of::<FVec>()));
        unsafe { _mm512_load_ps(ptr.cast()) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_u8(dst: *mut u8, data: IVec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm512_store_si512(dst.cast(), data) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_i32(dst: *mut i32, data: IVec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm512_store_si512(dst.cast(), data) }
    }

    /// Stores a vector at the given pointer.
    pub fn to_ptr_f32(dst: *mut f32, data: FVec) {
        debug_assert!((dst as usize).is_multiple_of(std::mem::align_of::<IVec>()));
        unsafe { _mm512_store_ps(dst.cast(), data) }
    }

    /// Multiplies two vectors together and takes the high 16 bits.
    pub fn mulhi_i16(x: IVec, y: IVec) -> IVec {
        unsafe { _mm512_mulhi_epi16(x, y) }
    }

    /// Multiplies two vectors together.
    pub fn mul_f32(x: FVec, y: FVec) -> FVec {
        unsafe { _mm512_mul_ps(x, y) }
    }

    /// Multiplies x and y and adds to z.
    pub fn fmadd_f32(x: FVec, y: FVec, z: FVec) -> FVec {
        unsafe { _mm512_fmadd_ps(x, y, z) }
    }

    /// Returns min of two vectors.
    pub fn min_i16(x: IVec, y: IVec) -> IVec {
        unsafe { _mm512_min_epi16(x, y) }
    }

    /// Returns max of two vectors.
    pub fn max_i16(x: IVec, y: IVec) -> IVec {
        unsafe { _mm512_max_epi16(x, y) }
    }

    /// Clamps a vector between two values.
    pub fn clamp_i16(v: IVec, min: IVec, max: IVec) -> IVec {
        min_i16(max, max_i16(v, min))
    }

    /// Clamps a vector between two values.
    pub fn clamp_f32(v: FVec, min: FVec, max: FVec) -> FVec {
        unsafe { _mm512_min_ps(max, _mm512_max_ps(v, min)) }
    }

    /// Shift left by <SHIFT> and pad with 0s.
    /// HACK: Have to accommodate for avx2. Who wrote this interface
    pub type ShiftT = u32;
    pub fn shl_i16<const SHIFT: ShiftT>(v: IVec) -> IVec {
        unsafe { _mm512_slli_epi16(v, SHIFT) }
    }

    /// Convert packed i16s to u8s with unsigned saturation (0..255).
    pub fn packus_i16_u8(x: IVec, y: IVec) -> IVec {
        unsafe { _mm512_packus_epi16(x, y) }
    }

    /// Convert packed i32s -> f32s.
    pub fn cvt_i32_f32(x: IVec) -> FVec {
        unsafe { _mm512_cvtepi32_ps(x) }
    }

    /// Gets the sum of the values in the vector.
    pub fn reduce_add_f32(v: FVec) -> f32 {
        unsafe { _mm512_reduce_add_ps(v) }
    }

    /// Gets a mask of all the nonzero elements in the vector.
    pub fn nonzero_mask_i32(v: IVec) -> Mask16 {
        unsafe { _mm512_cmpgt_epi32_mask(v, zeroed_i()) }
    }

    /// Multiply groups of u8s -> i16s -> i32s and sum these with `sum`.
    pub fn dpbusd_i32(sum: IVec, x: IVec, y: IVec) -> IVec {
        unsafe { _mm512_dpbusd_epi32(sum, x, y) }
    }
}
