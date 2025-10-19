pub mod vi16 {
    use std::arch::x86_64::*;
    pub type SVec = __m512i;

    pub const CHUNK_SIZE_I16: usize = std::mem::size_of::<SVec>() / std::mem::size_of::<i16>();

    /// Returns a zeroed out vector.
    pub fn zeroed() -> SVec {
        unsafe { _mm512_setzero_si512() }
    }

    /// Returns a vector set to the given value.
    pub fn from_val(val: i16) -> SVec {
        unsafe { _mm512_set1_epi16(val) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    pub fn from_ptr(ptr: *const i16) -> SVec {
        unsafe { _mm512_load_si512(ptr.cast()) }
    }

    /// Multiplies two vectors together.
    pub fn mul16(x: SVec, y: SVec) -> SVec {
        unsafe { _mm512_mullo_epi16(x, y) }
    }

    /// Multiplies and Adds two vectors together.
    pub fn madd16(x: SVec, y: SVec) -> SVec {
        unsafe { _mm512_madd_epi16(x, y) }
    }

    /// Adds two vectors together in i32 space.
    pub fn add32(x: SVec, y: SVec) -> SVec {
        unsafe { _mm512_add_epi32(x, y) }
    }

    /// Clamps a vector between two values.
    pub fn clamp16(v: SVec, min: SVec, max: SVec) -> SVec {
        unsafe { _mm512_min_epi16(max, _mm512_max_epi16(v, min)) }
    }

    /// Gets the sum of the values in the vector.
    pub fn sum32(v: SVec) -> i32 {
        unsafe { _mm512_reduce_add_epi32(v) }
    }
}
