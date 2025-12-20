use utils::memory::Align64;

use crate::{arch::L1, simd::simd};

#[repr(C, align(64))]
struct NonZeroIndicies {
    indicies: [[u16; 8]; 256],
}

impl NonZeroIndicies {
    pub fn get_idxs(&self, byte: simd::Mask32) -> v128::IVec {
        unsafe { v128::from_ptr_i16(self.indicies.as_ptr().add(byte as usize).cast()) }
    }
}

const NNZ_OFFSETS: NonZeroIndicies = {
    let mut table = [[0; 8]; 256];

    let mut i = 0;
    while i < 256 {
        let mut j = i;
        let mut k = 0;
        while j != 0 {
            table[i][k] = j.trailing_zeros() as u16;
            j &= j - 1;
            k += 1;
        }
        i += 1
    }

    NonZeroIndicies { indicies: table }
};

const NNZ_PER_CHUNK: usize = simd::CHUNK_SIZE_I32 / 4;

pub struct SparseMat {
    pub indices: Align64<[u16; L1 / 4]>,
    pub count: usize,

    base: v128::IVec,
}

impl Default for SparseMat {
    fn default() -> Self {
        SparseMat { indices: Align64([0; L1 / 4]), count: 0, base: v128::zeroed_i() }
    }
}

impl SparseMat {
    pub fn update(&mut self, x: simd::IVec, y: simd::IVec) {
        unsafe {
            let mask = simd::nonzero_mask_i32(x) as simd::Mask32 | (simd::nonzero_mask_i32(y) as simd::Mask32) << simd::CHUNK_SIZE_I32;

            let iptr = self.indices.as_mut_ptr();

            for i in 0..NNZ_PER_CHUNK {
                let byte = (mask >> (i * 8)) & 0xff;
                let nnz_idxs = NNZ_OFFSETS.get_idxs(byte);
                let offset_idxs = v128::add_i16(nnz_idxs, self.base);

                v128::to_ptr_u(iptr.add(self.count).cast(), offset_idxs);

                self.count += byte.count_ones() as usize;
                self.base = v128::add_i16(self.base, v128::from_val_i16(8));
            }
        }
    }

    pub fn index_for(&self, c: usize) -> usize {
        self.indices[c] as usize
    }
}

// We only use 128 wide simd here, and this will only be used if we're aiming to support a
// SIMD-compatible target.
// TODO: support SSE ..??
mod v128 {
    use std::arch::x86_64::*;
    pub type IVec = __m128i;

    pub fn zeroed_i() -> IVec {
        unsafe { _mm_setzero_si128() }
    }

    pub fn from_val_i16(val: i16) -> IVec {
        unsafe { _mm_set1_epi16(val) }
    }

    pub fn from_ptr_i16(ptr: *const i16) -> IVec {
        unsafe { _mm_load_si128(ptr.cast()) }
    }

    pub fn to_ptr_u(dst: *mut u16, data: IVec) {
        unsafe { _mm_storeu_si128(dst.cast(), data) }
    }

    pub fn add_i16(x: IVec, y: IVec) -> IVec {
        unsafe { _mm_add_epi16(x, y) }
    }
}
