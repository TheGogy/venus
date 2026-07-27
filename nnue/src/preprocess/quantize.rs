#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use utils::memory::boxed_zeroed;

use crate::arch::{FEATURES, FT_QUANT, L1_LEN, L1_QUANT, L2_LEN, NB_INPUT_BUCKETS, NB_OUTPUT_BUCKETS, QuantNNUEData, RawNNUEData};

// Quantize a single value.
fn quantize(v: f32, q: i32) -> i16 {
    const B: f32 = 1.98;

    if v.abs() > B {
        println!("Value exceeds bounds!!! {v} >= {B}");
    }

    (v.clamp(-B, B) * q as f32).round() as i16
}

impl RawNNUEData {
    /// Quantize a network from Bullet and save it in a format that we can use for inference.
    pub fn quantize(&self) -> Box<QuantNNUEData> {
        let mut out: Box<QuantNNUEData> = boxed_zeroed();

        // Quantize FT weights.
        println!("Quantizing FT weights...");
        for bkt in 0..NB_INPUT_BUCKETS {
            for feat in 0..L1_LEN * FEATURES {
                // Merge in factorizer.
                let v = self.ftw[bkt + 1][feat] + self.ftw[0][feat];
                out.ftw[bkt * (L1_LEN * FEATURES) + feat] = quantize(v, FT_QUANT);
            }
        }

        // Quantize FT biases.
        println!("Quantizing FT biases...");
        for i in 0..L1_LEN {
            out.ftb[i] = quantize(self.ftb[i], FT_QUANT);
        }

        // Quantize L1 weights.
        println!("Quantizing L1 biases...");
        for b in 0..NB_OUTPUT_BUCKETS {
            for i in 0..L1_LEN {
                for j in 0..L2_LEN {
                    out.l1w[i][b][j] = quantize(self.l1w[i][b][j], L1_QUANT) as i8;
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
