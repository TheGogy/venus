use utils::memory::Align64;

use crate::{
    features::{
        NB_OUTPUT_BUCKETS,
        pawn::PAWN_FEATURES,
        psqt::{NB_INPUT_BUCKETS, PSQT_FEATURES},
        threat::THRT_FEATURES,
    },
    simd,
};

/// Quantization factors.
pub const SCALE: f32 = 400.0;
pub const FT_QUANT: i32 = 255;
pub const L1_QUANT: i32 = 128;

/// How far right the pairwise product is shifted before `packus` puts it in a `u8`.
pub const L1Q_SHIFT: simd::ShiftT = if simd::HAS_USDOT { 8 } else { 9 };

/// Invert the quantization steps: both feature transform halves carry a factor of `FT_QUANT`, the
/// pairwise product gives back `L1Q_SHIFT` bits, and the L1 weights carry `L1_QUANT`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub const L1_DEQUANT: f32 = (1 << L1Q_SHIFT) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

/// Input bucket map.
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

/// Rows in each feature transform.
pub const FT_THRT_ROWS: usize = PAWN_FEATURES + THRT_FEATURES;
pub const FT_PSQT_ROWS: usize = PSQT_FEATURES * NB_INPUT_BUCKETS;

/// Layer sizes.
pub const L1_LEN: usize = 1024;
pub const L2_LEN: usize = 32;
pub const L3_LEN: usize = 32;

const _: () = assert!(L1_LEN.is_multiple_of(simd::I16_LANES));

/// Helper type for an accumulator for each side.
pub type HalfAcc = Align64<[i16; L1_LEN]>;

/// Length of L1 for each side.
pub const PAIRWISE_LEN: usize = L1_LEN / 2;

/// L2 architecture has first half CReLU, second half squared and then CReLU.
pub const EFF_L2_LEN: usize = L2_LEN * 2;

/// L3 takes both halves of the L1 output and L2's output.
pub const EFF_L3_LEN: usize = EFF_L2_LEN + L3_LEN;

/// Weights and biases for the NNUE ready for inference.
#[repr(C)]
#[rustfmt::skip]
pub struct NNUEData {
    pub ftw_psqt: [Align64<[i16; L1_LEN]>; FT_PSQT_ROWS],
    pub ftw_thrt: [Align64<[i8 ; L1_LEN]>; FT_THRT_ROWS],

    pub ftb:  Align64<[i16; L1_LEN]>,
    pub l1w: [Align64<[i8 ; L1_LEN *     L2_LEN]>; NB_OUTPUT_BUCKETS],
    pub l1b: [Align64<[f32; L2_LEN]>;              NB_OUTPUT_BUCKETS],
    pub l2w: [Align64<[f32; EFF_L2_LEN * L3_LEN]>; NB_OUTPUT_BUCKETS],
    pub l2b: [Align64<[f32; L3_LEN]>;              NB_OUTPUT_BUCKETS],
    pub l3w: [Align64<[f32; EFF_L3_LEN]>;          NB_OUTPUT_BUCKETS],
    pub l3b: [f32;                                 NB_OUTPUT_BUCKETS],
}

/// Weights and biases for the NNUE as embedded in the executable.
#[repr(C)]
#[rustfmt::skip]
pub struct EmbedNNUEData {
    pub ftw_psqt:   [i16; L1_LEN * FT_PSQT_ROWS],
    pub ftw_thrt:   [i8 ; L1_LEN * FT_THRT_ROWS],

    pub ftb:   [i16; L1_LEN],
    pub l1w: [[[i8 ; L2_LEN]; NB_OUTPUT_BUCKETS]; L1_LEN],
    pub l1b:  [[f32; L2_LEN]; NB_OUTPUT_BUCKETS],
    pub l2w: [[[f32; L3_LEN]; NB_OUTPUT_BUCKETS]; EFF_L2_LEN],
    pub l2b:  [[f32; L3_LEN]; NB_OUTPUT_BUCKETS],
    pub l3w:  [[f32; NB_OUTPUT_BUCKETS]; EFF_L3_LEN],
    pub l3b:   [f32; NB_OUTPUT_BUCKETS],
}
