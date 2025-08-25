use ::utils::memory::Align64;

pub mod propagate;
pub mod utils;

// Quantization factors.
pub const QA: i32 = 255;
pub const QB: i32 = 64;
pub const QAB: i32 = QA * QB;

// Scaling factor.
pub const SCALE: i32 = 400;

// Layer sizes.
// These should ideally be divisible by Simd::CHUNK_SIZE.
pub const FEATURES: usize = 768;
pub const L1: usize = 2048;

// King bucket map (horizontally mirrored).
#[rustfmt::skip]
const BUCKET_MAP: [usize; 64] = [
    0,  1,  2,  3, 13, 12, 11, 10,
    4,  4,  5,  5, 15, 15, 14, 14,
    6,  6,  6,  6, 16, 16, 16, 16,
    7,  7,  7,  7, 17, 17, 17, 17,
    8,  8,  8,  8, 18, 18, 18, 18,
    8,  8,  8,  8, 18, 18, 18, 18,
    9,  9,  9,  9, 19, 19, 19, 19,
    9,  9,  9,  9, 19, 19, 19, 19,
];

pub const NB_INPUT_BUCKETS: usize = 10;
pub const NB_OUTPUT_BUCKETS: usize = 8;

/// Weights and biases for the NNUE.
#[repr(C)]
#[rustfmt::skip]
pub struct NNUEData {
    pub feature_weights:  Align64<[i16; L1 * FEATURES * NB_INPUT_BUCKETS]>,
    pub feature_bias:     Align64<[i16; L1]>,
    pub output_weights: [[Align64<[i16; L1]>; 2]; NB_OUTPUT_BUCKETS],
    pub output_bias:      Align64<[i16; NB_OUTPUT_BUCKETS]>,
}
