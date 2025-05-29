use ::utils::Align64;

pub mod propagate;
pub mod utils;

mod flatten;

// Quantization factors.
pub const QA: i32 = 255;
pub const QB: i32 = 64;
pub const QAB: i32 = QA * QB;

// Scaling factor.
pub const SCALE: i32 = 400;

// Layer sizes.
// These should ideally be divisible by Simd::CHUNK_SIZE.
pub const FEATURES: usize = 768;
pub const L1: usize = 1024;

/// Weights and biases for the NNUE.
#[repr(C)]
#[rustfmt::skip]
pub struct NNUEData {
    pub feature_weights: Align64<[i16; L1 * FEATURES]>,
    pub feature_bias:    Align64<[i16; L1]>,
    pub output_weights: [Align64<[i16; L1]>; 2],
    pub output_bias: i16,
}
