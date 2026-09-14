use std::mem::MaybeUninit;
#[cfg(feature = "nnz_logging")]
use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Seek, SeekFrom, Write},
};

use utils::memory::Align64;

#[cfg(feature = "nnz_logging")]
use crate::arch::PAIRWISE_LEN;
#[cfg(feature = "nnz_logging")]
use crate::preprocess::ftperm::USE_FTPERM;
use crate::{arch::L1_LEN, simd};

/// List of nonzero indices.
pub struct SparseMat {
    indices: Align64<[MaybeUninit<u16>; L1_LEN / 4]>,
}

impl Default for SparseMat {
    fn default() -> Self {
        Self { indices: Align64([MaybeUninit::uninit(); L1_LEN / 4]) }
    }
}

#[derive(Clone, Copy)]
pub struct NnzState {
    pub count: usize,
    base: nnz::Base,
}

impl Default for NnzState {
    fn default() -> Self {
        Self { count: 0, base: nnz::init() }
    }
}

// `nnz::push` writes a whole vector of indices at a time, so the list has to divide evenly into
// those chunks for the last write of a fully dense layer to land inside it.
const _: () = assert!((L1_LEN / 4).is_multiple_of(nnz::STRIDE));

impl SparseMat {
    /// Append the indices of the nonzero 4-byte groups of `x` and `y` to the index list.
    pub fn update(&mut self, st: &mut NnzState, x: simd::I32Vec, y: simd::I32Vec) {
        let mask = simd::nonzero_mask_i32(x) | simd::nonzero_mask_i32(y) << simd::I32_LANES;
        unsafe { nnz::push(self.indices.as_mut_ptr().cast(), &mut st.count, &mut st.base, mask) }
    }

    /// # Safety
    /// `c` must be less than the final [`NnzState::count`] reached while building the list.
    pub unsafe fn index_for(&self, c: usize) -> usize {
        unsafe { self.indices.get_unchecked(c).assume_init() as usize }
    }
}

/// Turning a nonzero-lane mask into the list of lane indices it names.
#[cfg(target_feature = "avx512vbmi2")]
mod nnz {
    #[allow(clippy::wildcard_imports)]
    use std::arch::x86_64::*;

    use crate::simd;

    /// The lane index each `u16` slot would contribute, for the chunk currently being scanned.
    pub type Base = __m512i;

    /// How many lanes one [`push`] covers.
    pub const STRIDE: usize = 2 * simd::I32_LANES;

    #[rustfmt::skip]
    pub fn init() -> Base {
        unsafe {
            _mm512_set_epi16(
                31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16,
                15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,
            )
        }
    }

    /// # Safety
    /// `dst.add(*count)` must have room for [`STRIDE`] `u16`s: the compressed vector is stored
    /// whole, and only `count` is advanced by the number of lanes that were actually set.
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub unsafe fn push(dst: *mut u16, count: &mut usize, base: &mut Base, mask: simd::Mask32) {
        unsafe {
            _mm512_storeu_si512(dst.add(*count).cast(), _mm512_maskz_compress_epi16(mask, *base));
            *count += mask.count_ones() as usize;
            *base = _mm512_add_epi16(*base, _mm512_set1_epi16(STRIDE as i16));
        }
    }
}

#[cfg(not(target_feature = "avx512vbmi2"))]
mod nnz {
    use utils::cfor;

    use crate::simd;

    /// For each byte of a mask, the indices of its set bits, packed to the front.
    #[repr(C, align(64))]
    struct NonZeroIndices {
        indices: [[u16; 8]; 256],
    }

    #[allow(clippy::cast_possible_truncation)]
    const NNZ_OFFSETS: NonZeroIndices = {
        let mut table = [[0; 8]; 256];

        cfor!(let mut i = 0; i < 256; i += 1; {
            let mut j = i;
            let mut k = 0;
            while j != 0 {
                table[i][k] = j.trailing_zeros() as u16;
                j &= j - 1;
                k += 1;
            }
        });

        NonZeroIndices { indices: table }
    };

    pub type Base = v128::U16Vec128;

    /// How many lanes one [`push`] covers.
    pub const STRIDE: usize = 2 * simd::I32_LANES;

    pub fn init() -> Base {
        v128::splat_u16(0)
    }

    /// # Safety
    /// `dst.add(*count)` must have room for 8 more `u16`s.
    pub unsafe fn push(dst: *mut u16, count: &mut usize, base: &mut Base, mask: simd::Mask32) {
        unsafe {
            for i in 0..STRIDE / 8 {
                let byte = (mask >> (i * 8)) & 0xFF;
                let idxs = v128::load_u16(NNZ_OFFSETS.indices.as_ptr().add(byte as usize).cast());

                v128::store_u16(dst.add(*count), v128::add_u16(idxs, *base));

                *count += byte.count_ones() as usize;
                *base = v128::add_u16(*base, v128::splat_u16(8));
            }
        }
    }

    #[cfg(not(target_feature = "neon"))]
    mod v128 {
        use std::arch::x86_64::{__m128i, _mm_add_epi16, _mm_load_si128, _mm_set1_epi16, _mm_storeu_si128};
        pub type U16Vec128 = __m128i;

        pub fn splat_u16(val: i16) -> U16Vec128 {
            unsafe { _mm_set1_epi16(val) }
        }

        /// # Safety
        /// `ptr` must be valid for a read of one vector, and aligned to it.
        pub unsafe fn load_u16(ptr: *const u16) -> U16Vec128 {
            unsafe { _mm_load_si128(ptr.cast()) }
        }

        /// # Safety
        /// `dst` must be valid for a write of one vector.
        pub unsafe fn store_u16(dst: *mut u16, data: U16Vec128) {
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

        pub fn splat_u16(val: u16) -> U16Vec128 {
            unsafe { vdupq_n_u16(val) }
        }

        /// # Safety
        /// `ptr` must be valid for a read of one vector, and aligned to it.
        pub unsafe fn load_u16(ptr: *const u16) -> U16Vec128 {
            unsafe { vld1q_u16(ptr.cast()) }
        }

        /// # Safety
        /// `dst` must be valid for a write of one vector.
        pub unsafe fn store_u16(dst: *mut u16, data: U16Vec128) {
            unsafe { vst1q_u16(dst.cast(), data) }
        }

        pub fn add_u16(x: U16Vec128, y: U16Vec128) -> U16Vec128 {
            unsafe { vaddq_u16(x, y) }
        }
    }
}

#[cfg(feature = "nnz_logging")]
pub const ACTS_MAGIC: &[u8; 8] = b"NNZACT01";
#[cfg(feature = "nnz_logging")]
pub const ACTS_DUMP_FILE: &str = "acts.bin";

/// Records which pairwise neurons fired at each position.
#[cfg(feature = "nnz_logging")]
pub struct NNZPermTracker {
    chunk: [[u64; PAIRWISE_LEN]; 2],
    positions: usize,

    pub count: usize,
    pub total: usize,

    pub dump_file: BufWriter<File>,
}

#[cfg(feature = "nnz_logging")]
impl Default for NNZPermTracker {
    fn default() -> Self {
        let mut dump_file = BufWriter::new(File::create(ACTS_DUMP_FILE).unwrap());
        // Magic.
        dump_file.write_all(ACTS_MAGIC).unwrap();
        // No. of positions.
        dump_file.write_all(&0u64.to_le_bytes()).unwrap();
        Self { count: 0, total: 0, chunk: [[0; PAIRWISE_LEN]; 2], positions: 0, dump_file }
    }
}

#[cfg(feature = "nnz_logging")]
impl NNZPermTracker {
    /// Track the current nonzero indices.
    pub fn update(&mut self, ft_out: &Align64<[u8; L1_LEN]>, sparse_count: usize) {
        for (i, &act) in ft_out.iter().enumerate() {
            self.chunk[i / PAIRWISE_LEN][i % PAIRWISE_LEN] |= (u64::from(act != 0) << self.positions % 64);
        }

        self.positions += 1;
        if self.positions.is_multiple_of(64) {
            self.flush_chunk();
        }

        self.count += sparse_count;
        self.total += L1_LEN / 4;
    }

    /// Flush the current chunk to the output file.
    fn flush_chunk(&mut self) {
        let chunk = std::mem::replace(&mut self.chunk, [[0; PAIRWISE_LEN]; 2]);
        for half in &chunk {
            for word in half {
                self.dump_file.write_all(&word.to_le_bytes()).unwrap();
            }
        }
    }

    /// Dump logs to file for processing.
    pub fn dump_stats(&mut self) -> Result<(), std::io::Error> {
        println!("Acts done:  {}", self.count);
        println!("Total acts: {}", self.total);
        println!("NNZ ratio:  {:.5}", self.count as f64 / self.total as f64);

        if USE_FTPERM {
            println!("Indices permuted! Activations will be incorrect.");
            return Ok(());
        }

        // Flush data if we haven't written it.
        if !self.positions.is_multiple_of(64) {
            self.flush_chunk();
        }

        self.dump_file.seek(SeekFrom::Start(ACTS_MAGIC.len() as u64))?;
        self.dump_file.write_all(&(self.positions as u64).to_le_bytes())?;
        self.dump_file.flush()?;
        println!("Wrote {} positions of activations to acts.bin.", self.positions);

        Ok(())
    }
}

// WARN: NOT MULTITHREADED!!!
#[cfg(feature = "nnz_logging")]
thread_local! {
    pub static NNZ_TRACKER: RefCell<NNZPermTracker> = RefCell::new(NNZPermTracker::default());
}
