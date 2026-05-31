#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use utils::memory::boxed_zeroed;

use crate::arch::{
    FEATURES, FT_QUANT, HAS_FACTORIZER, L1_LEN, L1_QUANT, L2_LEN, NB_INPUT_BUCKETS, NB_OUTPUT_BUCKETS, QuantNNUEData, RawNNUEData,
};

impl RawNNUEData {
    /// Quantize a network from Bullet and save it in a format that we can use for inference.
    pub fn quantize(&self) -> Box<QuantNNUEData> {
        let mut out: Box<QuantNNUEData> = boxed_zeroed();

        // Quantize FT weights.
        for bkt in 0..NB_INPUT_BUCKETS {
            for feat in 0..L1_LEN * FEATURES {
                // Add in the feature factorizer if we're using it.
                let v = if HAS_FACTORIZER { self.ftw[bkt + 1][feat] + self.ftw[0][feat] } else { self.ftw[bkt][feat] };
                out.ftw[bkt * (L1_LEN * FEATURES) + feat] = (v * FT_QUANT as f32).round() as i16;
            }
        }

        // Quantize FT biases.
        for i in 0..L1_LEN {
            out.ftb[i] = (self.ftb[i] * FT_QUANT as f32).round() as i16;
        }

        // Quantize L1 weights.
        for b in 0..NB_OUTPUT_BUCKETS {
            for i in 0..L1_LEN {
                for j in 0..L2_LEN {
                    out.l1w[i][b][j] = (self.l1w[i][b][j] * L1_QUANT as f32).round() as i8;
                }
            }
        }

        // Layers 2 and 3 are in full precision.
        out.l1b = self.l1b;
        out.l2w = self.l2w;
        out.l2b = self.l2b;
        out.l3w = self.l3w;
        out.l3b = self.l3b;

        out
    }
}
