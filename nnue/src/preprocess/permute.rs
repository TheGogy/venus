use utils::memory::{Align64, boxed_zeroed};

use crate::{arch::*, simd::simd};

impl QuantNNUEData {
    /// Helper function to permute ft weights/biases for packus.
    /// Only used if we're going to be using manual SIMD.
    ///
    /// Packus interleaves each block of 128 from a and b, but we want them
    /// to be consecutive - so we un-interleave them now so that they'll be properly concatenated.
    #[allow(clippy::needless_range_loop)]
    fn permute_packus(&self, out: &mut Box<NNUEData>) {
        const PACKUS_CHUNK: usize = 8;
        let mut regs = [[0i16; PACKUS_CHUNK]; simd::NB_PACKUS_REGS];

        let mut permute_chunk = |src: &[i16], dst: &mut Align64<[i16; L1]>, base: usize| {
            // Read chunks in order.
            for j in 0..simd::NB_PACKUS_REGS {
                let start = base + j * PACKUS_CHUNK;
                regs[j].copy_from_slice(&src[start..start + PACKUS_CHUNK]);
            }
            // Write chunks according to PACKUS_ORDER.
            for j in 0..simd::NB_PACKUS_REGS {
                let start = base + j * PACKUS_CHUNK;
                dst[start..start + PACKUS_CHUNK].copy_from_slice(&regs[simd::PACKUS_ORDER[j]]);
            }
        };

        // Repermute feature transform weights.
        for feat in 0..NB_INPUT_BUCKETS * FEATURES {
            for i in (0..L1 / PACKUS_CHUNK).step_by(simd::NB_PACKUS_REGS) {
                permute_chunk(&self.ftw[feat * L1..], &mut out.ftw[feat], i * PACKUS_CHUNK);
            }
        }

        // Repermute feature transform bias.
        for i in (0..L1 / PACKUS_CHUNK).step_by(simd::NB_PACKUS_REGS) {
            permute_chunk(&self.ftb, &mut out.ftb, i * PACKUS_CHUNK);
        }
    }

    /// Repermute the NNUE to a format helpful for SIMD.
    #[allow(unreachable_code, unused_variables)]
    pub fn permute(&self) -> Box<NNUEData> {
        let mut out: Box<NNUEData> = boxed_zeroed();

        #[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
        self.permute_packus(&mut out);

        // Transpose L1 weights.
        #[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
        for b in 0..NB_OUTPUT_BUCKETS {
            for i in (0..L1).step_by(4) {
                for j in 0..L2 {
                    for k in 0..4 {
                        out.l1w[b][i * L2 + j * 4 + k] = self.l1w[i + k][b][j];
                    }
                }
            }
        }

        #[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
        for b in 0..NB_OUTPUT_BUCKETS {
            for i in 0..L1 {
                for j in 0..L2 {
                    out.l1w[b][i * L2 + j] = self.l1w[i][b][j];
                }
            }
        }

        // The rest of the positions are the same for manual SIMD and autovec.
        for b in 0..NB_OUTPUT_BUCKETS {
            // Copy over L1 biases.
            for i in 0..L2 {
                out.l1b[b][i] = self.l1b[b][i];
            }

            // Transpose L2 weights.
            for i in 0..L2 {
                for j in 0..L3 {
                    out.l2w[b][i * L3 + j] = self.l2w[i][b][j];
                }
            }

            // Copy over L2 biases.
            for i in 0..L3 {
                out.l2b[b][i] = self.l2b[b][i];
            }

            // Transpose L3 weights.
            for i in 0..L3 {
                out.l3w[b][i] = self.l3w[i][b];
            }

            // Copy over L3 biases.
            out.l3b[b] = self.l3b[b];
        }

        out
    }
}
