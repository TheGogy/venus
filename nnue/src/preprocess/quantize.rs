#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use utils::memory::boxed_zeroed;

use crate::{
    arch::{FT_QUANT, FT_THRT_ROWS, L1_LEN, L1_QUANT, L2_LEN, QuantNNUEData, RawNNUEData},
    features::{
        NB_OUTPUT_BUCKETS,
        psqt::{NB_INPUT_BUCKETS, PSQT_FEATURES},
    },
};

/// Quantize one weight by `q` into a `bound`-wide integer, reporting anything that had to clamp.
fn quantize(v: f32, q: i32, bound: i32) -> i32 {
    let quant = (v * q as f32).round() as i32;
    if quant < -bound - 1 || quant > bound {
        println!("Value exceeds bounds!!! {v} quantizes to {quant}, past +/-{bound}");
    }
    quant.clamp(-bound - 1, bound)
}

fn quantize_i8(v: f32, q: i32) -> i8 {
    quantize(v, q, i32::from(i8::MAX)) as i8
}

fn quantize_i16(v: f32, q: i32) -> i16 {
    quantize(v, q, i32::from(i16::MAX)) as i16
}

impl RawNNUEData {
    /// Quantize a network from Bullet and save it in a format that we can use for inference.
    pub fn quantize(&self) -> Box<QuantNNUEData> {
        let mut out: Box<QuantNNUEData> = boxed_zeroed();

        println!("Quantizing FT Threat weights...");
        for i in 0..L1_LEN * FT_THRT_ROWS {
            out.ftw_thrt[i] = quantize_i8(self.ftw_thrt[i], FT_QUANT);
        }

        println!("Quantizing FT PSQT weights...");
        for bkt in 0..NB_INPUT_BUCKETS {
            for feat in 0..L1_LEN * PSQT_FEATURES {
                out.ftw_psqt[bkt * (L1_LEN * PSQT_FEATURES) + feat] = quantize_i16(self.ftw_psqt[bkt][feat], FT_QUANT);
            }
        }

        println!("Quantizing FT biases...");
        for i in 0..L1_LEN {
            out.ftb[i] = quantize_i16(self.ftb[i], FT_QUANT);
        }

        println!("Quantizing L1 weights...");
        for b in 0..NB_OUTPUT_BUCKETS {
            for i in 0..L1_LEN {
                for j in 0..L2_LEN {
                    out.l1w[i][b][j] = quantize_i8(self.l1w[i][b][j], L1_QUANT);
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
