pub const CHUNK_SIZE: usize = 32;

pub mod vi16 {
    use std::arch::x86_64::*;
    pub type Vec = __m512i;

    /// Returns a zeroed out vector.
    #[inline]
    pub fn zeroed() -> Vec {
        unsafe { _mm512_setzero_si512() }
    }

    /// Returns a vector set to the given value.
    #[inline]
    pub fn from_val(val: i16) -> Vec {
        unsafe { _mm512_set1_epi16(val) }
    }

    /// Loads a vector in directly from the values at the given pointer.
    #[inline]
    pub fn from_ptr(ptr: *const i16) -> Vec {
        unsafe { _mm512_load_si512(ptr.cast()) }
    }

    /// Multiplies two vectors together.
    #[inline]
    pub fn mul(x: Vec, y: Vec) -> Vec {
        unsafe { _mm512_mullo_epi16(x, y) }
    }

    /// Multiplies and Adds two vectors together.
    #[inline]
    pub fn madd(x: Vec, y: Vec) -> Vec {
        unsafe { _mm512_madd_epi16(x, y) }
    }

    /// Adds two vectors together.
    #[inline]
    pub fn add(x: Vec, y: Vec) -> Vec {
        unsafe { _mm512_add_epi32(x, y) }
    }

    /// Clamps a vector between two values.
    #[inline]
    pub fn clamp(v: Vec, min: Vec, max: Vec) -> Vec {
        unsafe { _mm512_min_epi16(max, _mm512_max_epi16(v, min)) }
    }

    /// Gets the sum of the values in the vector.
    #[inline]
    pub fn sum(v: Vec) -> i32 {
        unsafe { _mm512_reduce_add_epi32(v) }
    }
}
