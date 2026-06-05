#[cfg(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon"))]
use utils::memory::Align64;
use utils::memory::boxed_zeroed;

use crate::arch::{EFF_L2_LEN, FEATURES, L1_LEN, L2_LEN, L3_LEN, NB_INPUT_BUCKETS, NB_OUTPUT_BUCKETS, NNUEData, QuantNNUEData};
#[cfg(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon"))]
use crate::simd::simd;

impl QuantNNUEData {
    /// Helper function to permute ft weights/biases for packus.
    /// Only used if we're going to be using manual SIMD.
    ///
    /// Packus interleaves each block of 128 from a and b, but we want them
    /// to be consecutive - so we un-interleave them now so that they'll be properly concatenated.
    #[allow(clippy::needless_range_loop)]
    #[cfg(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon"))]
    fn permute_packus(&self, out: &mut Box<NNUEData>) {
        const PACKUS_CHUNK: usize = 8;

        let permute_chunk = |src: &[i16], dst: &mut Align64<[i16; L1_LEN]>, base: usize| {
            for (d, &s) in simd::PACKUS_ORDER.iter().enumerate() {
                let src_idx = base + s * PACKUS_CHUNK;
                let dst_idx = base + d * PACKUS_CHUNK;
                dst[dst_idx..dst_idx + PACKUS_CHUNK].copy_from_slice(&src[src_idx..src_idx + PACKUS_CHUNK]);
            }
        };

        // Permute feature transform weights.
        for feat in 0..NB_INPUT_BUCKETS * FEATURES {
            for i in (0..L1_LEN / PACKUS_CHUNK).step_by(simd::NB_PACKUS_REGS) {
                permute_chunk(&self.ftw[feat * L1_LEN..], &mut out.ftw[feat], i * PACKUS_CHUNK);
            }
        }

        // Permute feature transform bias.
        for i in (0..L1_LEN / PACKUS_CHUNK).step_by(simd::NB_PACKUS_REGS) {
            permute_chunk(&self.ftb, &mut out.ftb, i * PACKUS_CHUNK);
        }
    }

    /// Repermute the NNUE to a format helpful for SIMD.
    #[allow(unreachable_code, unused_variables)]
    pub fn permute(&self) -> Box<NNUEData> {
        let mut out: Box<NNUEData> = boxed_zeroed();

        #[cfg(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon"))]
        self.permute_packus(&mut out);

        #[cfg(not(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon")))]
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.ftw.as_ptr() as *const i16,
                out.ftw.as_mut_ptr() as *mut i16,
                L1_LEN * FEATURES * NB_INPUT_BUCKETS,
            );
            out.ftb.copy_from_slice(&self.ftb);
        }

        for b in 0..NB_OUTPUT_BUCKETS {
            // Transpose L1 weights.
            #[cfg(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon"))]
            for i in (0..L1_LEN).step_by(4) {
                for j in 0..L2_LEN {
                    for k in 0..4 {
                        out.l1w[b][i * L2_LEN + j * 4 + k] = self.l1w[i + k][b][j];
                    }
                }
            }

            // Transpose L1 weights.
            #[cfg(not(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon")))]
            for i in 0..L1_LEN {
                for j in 0..L2_LEN {
                    out.l1w[b][i * L2_LEN + j] = self.l1w[i][b][j];
                }
            }

            // Transpose L2 weights.
            for i in 0..EFF_L2_LEN {
                for j in 0..L3_LEN {
                    out.l2w[b][i * L3_LEN + j] = self.l2w[i][b][j];
                }
            }

            // Transpose L3 weights.
            for i in 0..L3_LEN {
                out.l3w[b][i] = self.l3w[i][b];
            }
        }

        // Copy in the biases.
        for b in 0..NB_OUTPUT_BUCKETS {
            out.l1b[b].copy_from_slice(&self.l1b[b]);
            out.l2b[b].copy_from_slice(&self.l2b[b]);
            out.l3b[b] = self.l3b[b];
        }

        out
    }
}
