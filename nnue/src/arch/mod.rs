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
pub const L1: usize = 1536;

// King bucket map (horizontally mirrored).
#[rustfmt::skip]
const BUCKET_MAP: [usize; 64] = [
    0,  0,  1,  1,  9,  9,  8,  8,
    2,  2,  3,  3, 11, 11, 10, 10,
    4,  4,  5,  5, 13, 13, 12, 12,
    6,  6,  7,  7, 15, 15, 14, 14,
    6,  6,  7,  7, 15, 15, 14, 14,
    6,  6,  7,  7, 15, 15, 14, 14,
    6,  6,  7,  7, 15, 15, 14, 14,
    6,  6,  7,  7, 15, 15, 14, 14,
];

pub const BUCKET_NUM: usize = 8;

/// Weights and biases for the NNUE.
#[repr(C)]
#[rustfmt::skip]
pub struct NNUEData {
    pub feature_weights: Align64<[i16; L1 * FEATURES * BUCKET_NUM]>,
    pub feature_bias:    Align64<[i16; L1]>,
    pub output_weights: [Align64<[i16; L1]>; 2],
    pub output_bias: i16,
}
