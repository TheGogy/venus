use chess::types::{color::Color, piece::Piece, square::Square};
use utils::{max, memory::Align64};

use crate::{simd::simd, utils::make_bucket_map};

/// Quantization factors.
pub const SCALE: i32 = 400;
pub const FT_QUANT: i32 = 255;
pub const L1_QUANT: i32 = 64;

/// HACK: This should just be a u32 everywhere, but avx2 decided to be special
#[allow(clippy::cast_sign_loss)]
pub const L1Q_BITS: simd::ShiftT = L1_QUANT.trailing_zeros() as simd::ShiftT;
pub const L1Q_SHIFT: simd::ShiftT = 16 - L1Q_BITS;

/// Invert quantization steps (clamp by `FT_QUANT`, downscale by `FT_SHIFT`, quantize by `FT_QUANT * L1_QUANT`).
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub const L1_DEQUANT: f32 = (1 << L1Q_SHIFT) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

/// Whether to perform FT permutation.
pub const USE_FTPERM: bool = true;

/// Whether the unquantized output has a king bucket factorizer.
pub const HAS_FACTORIZER: bool = true;

/// Total input features.
pub const FEATURES: usize = Color::NUM * Piece::NUM * Square::NUM;

/// Layer sizes.
pub const L1_LEN: usize = 1792;
pub const L2_LEN: usize = 32;
pub const L3_LEN: usize = 32;

const _: () = assert!(L1_LEN.is_multiple_of(simd::CHUNK_SIZE_I16));

/// Helper type for an accumulator for each side.
pub type HalfAcc = Align64<[i16; L1_LEN]>;

/// Length of L1 for each side.
pub const PAIRWISE_LEN: usize = L1_LEN / 2;

/// L2 architecture has first half normal, second half squared.
pub const EFF_L2_LEN: usize = L2_LEN * 2;

/// Input expert map.
#[rustfmt::skip]
pub const HALF_BUCKET_MAP: [usize; 32] = [
   0,  1,  2,  3,
   4,  5,  6,  7,
   8,  9, 10, 11,
   8,  9, 10, 11,
  12, 12, 13, 13,
  12, 12, 13, 13,
  14, 14, 15, 15,
  14, 14, 15, 15,
];

/// We use 32 King positions defined by [`BUCKET_MAP`].
/// If the King is on any of the E-H files, we mirror all the features.
pub const NB_INPUT_BUCKETS: usize = max!(HALF_BUCKET_MAP) + 1;
pub const INPUT_KING_POSNS: usize = NB_INPUT_BUCKETS * 2;
pub const NB_OUTPUT_BUCKETS: usize = 8;

/// Full input expert map.
pub const BUCKET_MAP: [usize; Square::NUM] = make_bucket_map(HALF_BUCKET_MAP, NB_INPUT_BUCKETS);

/// Weights and biases for the NNUE ready for inference.
#[repr(C)]
#[rustfmt::skip]
pub struct NNUEData {
    pub ftw: [Align64<[i16; L1_LEN]>; FEATURES * NB_INPUT_BUCKETS],
    pub ftb:  Align64<[i16; L1_LEN]>,
    pub l1w: [Align64<[i8 ; L1_LEN *     L2_LEN]>; NB_OUTPUT_BUCKETS],
    pub l1b: [Align64<[f32; L2_LEN]>;              NB_OUTPUT_BUCKETS],
    pub l2w: [Align64<[f32; EFF_L2_LEN * L3_LEN]>; NB_OUTPUT_BUCKETS],
    pub l2b: [Align64<[f32; L3_LEN]>;              NB_OUTPUT_BUCKETS],
    pub l3w: [Align64<[f32; L3_LEN]>;              NB_OUTPUT_BUCKETS],
    pub l3b: [f32;                                 NB_OUTPUT_BUCKETS],
}

/// Weights and biases for the NNUE, quantized and embedded in the executable.
#[repr(C)]
#[rustfmt::skip]
pub struct QuantNNUEData {
    pub ftw:   [i16; L1_LEN * FEATURES * NB_INPUT_BUCKETS],
    pub ftb:   [i16; L1_LEN],
    pub l1w: [[[i8 ; L2_LEN]; NB_OUTPUT_BUCKETS]; L1_LEN],
    pub l1b:  [[f32; L2_LEN]; NB_OUTPUT_BUCKETS],
    pub l2w: [[[f32; L3_LEN]; NB_OUTPUT_BUCKETS]; EFF_L2_LEN],
    pub l2b:  [[f32; L3_LEN]; NB_OUTPUT_BUCKETS],
    pub l3w:  [[f32; NB_OUTPUT_BUCKETS]; L3_LEN],
    pub l3b:   [f32; NB_OUTPUT_BUCKETS],
}

/// Raw output straight from Bullet.
#[repr(C)]
#[rustfmt::skip]
pub struct RawNNUEData {
    pub ftw:  [[f32; L1_LEN * FEATURES]; NB_INPUT_BUCKETS + (HAS_FACTORIZER as usize)],
    pub ftb:   [f32; L1_LEN],
    pub l1w: [[[f32; L2_LEN]; NB_OUTPUT_BUCKETS]; L1_LEN],
    pub l1b:  [[f32; L2_LEN]; NB_OUTPUT_BUCKETS],
    pub l2w: [[[f32; L3_LEN]; NB_OUTPUT_BUCKETS]; EFF_L2_LEN],
    pub l2b:  [[f32; L3_LEN]; NB_OUTPUT_BUCKETS],
    pub l3w:  [[f32; NB_OUTPUT_BUCKETS]; L3_LEN],
    pub l3b:   [f32; NB_OUTPUT_BUCKETS],
}
