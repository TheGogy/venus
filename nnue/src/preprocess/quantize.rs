#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use utils::memory::boxed_zeroed;

use crate::arch::{FEATURES, FT_QUANT, L1, L1_QUANT, L2, NB_INPUT_BUCKETS, NB_OUTPUT_BUCKETS, QuantNNUEData, RawNNUEData};

impl RawNNUEData {
    /// Quantize a network from Bullet and save it in a format that we can use for inference.
    pub fn quantize(&self) -> Box<QuantNNUEData> {
        let mut out: Box<QuantNNUEData> = boxed_zeroed();

        // Quantize FT weights.
        for i in 0..L1 * FEATURES * NB_INPUT_BUCKETS {
            out.ftw[i] = (self.ftw[i] * FT_QUANT as f32).round() as i16;
        }

        // Quantize FT biases.
        for i in 0..L1 {
            out.ftb[i] = (self.ftb[i] * FT_QUANT as f32).round() as i16;
        }

        // Quantize L1 weights.
        for b in 0..NB_OUTPUT_BUCKETS {
            for i in 0..L1 {
                for j in 0..L2 {
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
