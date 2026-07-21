#[cfg(feature = "nnz_logging")]
use std::{cell::RefCell, fs::File, io::Write};

use utils::memory::Align64;

#[cfg(feature = "nnz_logging")]
use crate::arch::USE_FTPERM;
use crate::{arch::L1_LEN, simd::simd};

#[repr(C, align(64))]
struct NonZeroIndicies {
    indicies: [[u16; 8]; 256],
}

impl NonZeroIndicies {
    pub fn get_idxs(&self, byte: simd::Mask32) -> v128::U16Vec128 {
        unsafe { v128::from_ptr_u16(self.indicies.as_ptr().add(byte as usize).cast()) }
    }
}

#[allow(clippy::cast_possible_truncation)]
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
        i += 1;
    }

    NonZeroIndicies { indicies: table }
};

const NNZ_PER_CHUNK: usize = simd::CHUNK_SIZE_I32 / 4;

pub struct SparseMat {
    pub indices: Align64<[u16; L1_LEN / 4]>,
    pub count: usize,

    base: v128::U16Vec128,
}

impl Default for SparseMat {
    fn default() -> Self {
        Self { indices: Align64([0; L1_LEN / 4]), count: 0, base: v128::from_val_u16(0) }
    }
}

impl SparseMat {
    pub fn update(&mut self, x: simd::I32Vec, y: simd::I32Vec) {
        unsafe {
            let mask = simd::nonzero_mask_i32(x) | simd::nonzero_mask_i32(y) << simd::CHUNK_SIZE_I32;

            let iptr = self.indices.as_mut_ptr();

            for i in 0..NNZ_PER_CHUNK {
                let byte = (mask >> (i * 8)) & 0xFF;
                let nnz_idxs = NNZ_OFFSETS.get_idxs(byte);
                let offset_idxs = v128::add_u16(nnz_idxs, self.base);

                v128::to_ptr_u(iptr.add(self.count).cast(), offset_idxs);

                self.count += byte.count_ones() as usize;
                self.base = v128::add_u16(self.base, v128::from_val_u16(8));
            }
        }
    }

    pub fn index_for(&self, c: usize) -> usize {
        self.indices[c] as usize
    }
}

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
mod v128 {
    use std::arch::x86_64::{__m128i, _mm_add_epi16, _mm_load_si128, _mm_set1_epi16, _mm_storeu_si128};
    pub type U16Vec128 = __m128i;

    pub fn from_val_u16(val: i16) -> U16Vec128 {
        unsafe { _mm_set1_epi16(val) }
    }

    pub fn from_ptr_u16(ptr: *const u16) -> U16Vec128 {
        unsafe { _mm_load_si128(ptr.cast()) }
    }

    pub fn to_ptr_u(dst: *mut u16, data: U16Vec128) {
        unsafe { _mm_storeu_si128(dst.cast(), data) }
    }

    pub fn add_u16(x: U16Vec128, y: U16Vec128) -> U16Vec128 {
        unsafe { _mm_add_epi16(x, y) }
    }
}

#[cfg(target_feature = "neon")]
mod v128 {
    use std::arch::aarch64::{uint16x8_t, vaddq_u16, vdupq_n_u16, vld1q_u16, vst1q_u16};
    pub type U16Vec128 = uint16x8_t;

    pub fn from_val_u16(val: u16) -> U16Vec128 {
        unsafe { vdupq_n_u16(val) }
    }

    pub fn from_ptr_u16(ptr: *const u16) -> U16Vec128 {
        unsafe { vld1q_u16(ptr.cast()) }
    }

    pub fn to_ptr_u(dst: *mut u16, data: U16Vec128) {
        unsafe { vst1q_u16(dst.cast(), data) }
    }

    pub fn add_u16(x: U16Vec128, y: U16Vec128) -> U16Vec128 {
        unsafe { vaddq_u16(x, y) }
    }
}

#[cfg(feature = "nnz_logging")]
const PAIRWISE_LEN: usize = L1_LEN / 2;

#[cfg(feature = "nnz_logging")]
pub struct NNZPermTracker {
    pub coactivations: Box<[[u64; PAIRWISE_LEN]; PAIRWISE_LEN]>,

    pub count: usize,
    pub total: usize,

    pub dump_file: File,
}

#[cfg(feature = "nnz_logging")]
impl Default for NNZPermTracker {
    fn default() -> Self {
        Self {
            count: 0,
            total: 0,
            coactivations: vec![[0u64; PAIRWISE_LEN]; PAIRWISE_LEN].into_boxed_slice().try_into().unwrap(),
            dump_file: File::create("acts.bin").unwrap(),
        }
    }
}

#[cfg(feature = "nnz_logging")]
impl NNZPermTracker {
    /// Track the current nonzero indices.
    pub fn update(&mut self, ft_out: &Align64<[u8; L1_LEN]>, sparse_count: usize) {
        let mut counts = [0u64; PAIRWISE_LEN];
        let mut rec = [0u8; PAIRWISE_LEN];

        for (i, &act) in ft_out.iter().enumerate() {
            counts[i % PAIRWISE_LEN] += (act != 0) as u64;
            rec[i % PAIRWISE_LEN] += (act != 0) as u8;
        }

        for i in 0..PAIRWISE_LEN {
            if counts[i] != 0 {
                for j in 0..PAIRWISE_LEN {
                    self.coactivations[i][j] += counts[i] * counts[j];
                }
            }
        }

        self.dump_file.write_all(&rec).unwrap();
        self.count += sparse_count;
        self.total += L1_LEN / 4;
    }

    /// Dump logs to file for processing.
    pub fn dump_stats(&mut self) -> Result<(), std::io::Error> {
        println!("Acts done:  {}", self.count);
        println!("Total acts: {}", self.total);
        println!("Nnz ratio:  {:.5}", self.count as f64 / self.total as f64);

        if USE_FTPERM {
            println!("Indices permuted! Coactivations will be incorrect.");
            return Ok(());
        }

        println!("Writing full activations to acts.bin...");
        self.dump_file.flush().unwrap();

        println!("Writing nnz logs to coactivations.txt...");
        std::fs::write("coactivations.txt", format!("{:?}", self.coactivations))
    }
}

// WARN: NOT MULTITHREADED!!!
#[cfg(feature = "nnz_logging")]
thread_local! {
    pub static NNZ_TRACKER: RefCell<NNZPermTracker> = RefCell::new(NNZPermTracker::default());
}
