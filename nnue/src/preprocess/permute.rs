use utils::memory::boxed_zeroed;

use crate::{
    arch::{EFF_L2_LEN, EFF_L3_LEN, EmbedNNUEData, FT_PSQT_ROWS, FT_THRT_ROWS, L1_LEN, L2_LEN, L3_LEN, NNUEData},
    features::NB_OUTPUT_BUCKETS,
    simd,
};

/// Number of lanes packus keeps together.
const PACKUS_CHUNK: usize = 8;

const _: () = assert!(L1_LEN.is_multiple_of(simd::PACKUS_REGS * PACKUS_CHUNK));

/// Packus interleaves each block of [`PACKUS_CHUNK`] lanes from a and b, but we want them to be
/// consecutive - so we un-interleave a row now so that they'll be properly concatenated.
fn permute_row<T: Copy>(src: &[T], dst: &mut [T]) {
    for base in (0..L1_LEN).step_by(simd::PACKUS_REGS * PACKUS_CHUNK) {
        for (d, &s) in simd::PACKUS_ORDER.iter().enumerate() {
            let (si, di) = (base + s * PACKUS_CHUNK, base + d * PACKUS_CHUNK);
            dst[di..di + PACKUS_CHUNK].copy_from_slice(&src[si..si + PACKUS_CHUNK]);
        }
    }
}

impl EmbedNNUEData {
    /// Repermute the NNUE to a format helpful for SIMD.
    pub fn permute(&self) -> Box<NNUEData> {
        let mut out: Box<NNUEData> = boxed_zeroed();

        for feat in 0..FT_PSQT_ROWS {
            permute_row(&self.ftw_psqt[feat * L1_LEN..], &mut out.ftw_psqt[feat].0);
        }
        for feat in 0..FT_THRT_ROWS {
            permute_row(&self.ftw_thrt[feat * L1_LEN..], &mut out.ftw_thrt[feat].0);
        }
        permute_row(&self.ftb, &mut out.ftb.0);

        for b in 0..NB_OUTPUT_BUCKETS {
            // Transpose L1 weights.
            for i in (0..L1_LEN).step_by(4) {
                for j in 0..L2_LEN {
                    for k in 0..4 {
                        out.l1w[b][i * L2_LEN + j * 4 + k] = self.l1w[i + k][b][j];
                    }
                }
            }

            // Transpose L2 weights.
            for i in 0..EFF_L2_LEN {
                for j in 0..L3_LEN {
                    out.l2w[b][i * L3_LEN + j] = self.l2w[i][b][j];
                }
            }

            // Transpose L3 weights. These cover the L1 output and L2's output back to back.
            for i in 0..EFF_L3_LEN {
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
