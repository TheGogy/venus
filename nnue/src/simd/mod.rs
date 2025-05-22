pub use defs::*;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
mod defs {
    pub const CHUNK_SIZE: usize = 16;

    // Vector i16
    pub mod vi16 {
        use std::arch::x86_64::*;
        pub type Vec = __m256i;

        /// Returns a zeroed out vector.
        #[inline]
        pub fn zeroed() -> Vec {
            unsafe { _mm256_setzero_si256() }
        }

        /// Returns a vector set to the given value.
        #[inline]
        pub fn from_val(val: i32) -> Vec {
            unsafe { _mm256_set1_epi32(val) }
        }

        /// Loads a vector in directly from the values at the given pointer.
        #[inline]
        pub fn from_ptr(ptr: *const i16) -> Vec {
            unsafe { _mm256_load_epi32(ptr.cast()) }
        }

        /// Multiplies two vectors together.
        #[inline]
        pub fn mul(x: Vec, y: Vec) -> Vec {
            unsafe { _mm256_mullo_epi16(x, y) }
        }

        /// Multiplies and Adds two vectors together.
        #[inline]
        pub fn madd(x: Vec, y: Vec) -> Vec {
            unsafe { _mm256_madd_epi16(x, y) }
        }

        /// Adds two vectors together.
        #[inline]
        pub fn add(x: Vec, y: Vec) -> Vec {
            unsafe { _mm256_add_epi16(x, y) }
        }

        /// Clamps a vector between two values.
        #[inline]
        pub fn clamp(v: Vec, min: Vec, max: Vec) -> Vec {
            unsafe { _mm256_min_epi16(max, _mm256_max_epi16(v, min)) }
        }

        /// Gets the sum of the values in the vector.
        #[inline]
        pub fn sum(v: Vec) -> i32 {
            unsafe {
                let hi = _mm256_extracti128_si256::<1>(v);
                let lo = _mm256_castsi256_si128(v);
                let sum128 = _mm_add_epi32(hi, lo);
                let shuf64 = _mm_unpackhi_epi64(sum128, sum128);
                let sum64 = _mm_add_epi32(shuf64, sum128);
                let shuf32 = _mm_shuffle_epi32::<0b00_00_00_01>(sum64);
                let final_sum = _mm_add_epi32(sum64, shuf32);
                _mm_cvtsi128_si32(final_sum)
            }
        }
    }
}

#[cfg(target_feature = "avx512f")]
mod defs {
    pub const CHUNK_SIZE: usize = 32;

    // Vector i16
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
}
