use std::sync::OnceLock;

use ::utils::memory::Align64;
use chess::types::{color::Color, piece::Piece, square::Square};

use crate::simd::simd;

// Quantization factors.
pub const SCALE: i32 = 400;
pub const FT_QUANT: i32 = 255;
pub const L1_QUANT: i32 = 64;

// HACK: This should just be a u32 everywhere, but avx2 decided to be special
pub const L1Q_BITS: simd::ShiftT = L1_QUANT.trailing_zeros() as simd::ShiftT;

// Invert quantization steps (clamp by FT_QUANT, downscale by FT_SHIFT, quantize by FT_QUANT * L1_QUANT).
pub const L1_DEQUANT: f32 = (1 << (16 - L1Q_BITS)) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

// Layer sizes.
pub const FEATURES: usize = Color::NUM * Piece::NUM * Square::NUM;
pub const L1: usize = 1792;
pub const L2: usize = 16;
pub const L3: usize = 32;

const _: () = assert!(L1.is_multiple_of(simd::CHUNK_SIZE_I16));

// King bucket map.
// We only use the buckets on the A-D files, the E-H files are mirrored and go to the same bucket
// on the opposite side - but we want to store them separately in the finny table, so it is easier
// to store the bucket map like this.
#[rustfmt::skip]
pub const BUCKET_MAP: [usize; 64] = [
  0,  1,  2,  3,  19, 18, 17, 16,
  4,  5,  6,  7,  23, 22, 21, 20,
  8,  8,  9,  9,  25, 25, 24, 24,
  10, 10, 11, 11, 27, 27, 26, 26,
  12, 12, 13, 13, 29, 29, 28, 28,
  12, 12, 13, 13, 29, 29, 28, 28,
  14, 14, 15, 15, 31, 31, 30, 30,
  14, 14, 15, 15, 31, 31, 30, 30
];

/// We use 32 King positions defined by BUCKET_MAP.
/// If the King is on any of the E-H files, we mirror all the features.
pub const INPUT_KING_POSNS: usize = 32;
pub const NB_INPUT_BUCKETS: usize = INPUT_KING_POSNS / 2;
pub const NB_OUTPUT_BUCKETS: usize = 8;

/// Weights and biases for the NNUE ready for inference.
#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy)]
pub struct NNUEData {
    pub ftw: [Align64<[i16; L1]>; FEATURES * NB_INPUT_BUCKETS],
    pub ftb:  Align64<[i16; L1]>,
    pub l1w: [Align64<[i8 ; L1 * L2]>; NB_OUTPUT_BUCKETS],
    pub l1b: [Align64<[f32; L2]>;      NB_OUTPUT_BUCKETS],
    pub l2w: [Align64<[f32; L2 * L3]>; NB_OUTPUT_BUCKETS],
    pub l2b: [Align64<[f32; L3]>;      NB_OUTPUT_BUCKETS],
    pub l3w: [Align64<[f32; L3]>;      NB_OUTPUT_BUCKETS],
    pub l3b: [f32;                     NB_OUTPUT_BUCKETS],
}

/// Raw output straight from Bullet.
/// Factoriser merged in bullet output.
#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy)]
pub struct RawNNUEData {
    pub ftw:   [f32; L1 * FEATURES * NB_INPUT_BUCKETS],
    pub ftb:   [f32; L1],
    pub l1w: [[[f32; L2]; NB_OUTPUT_BUCKETS]; L1],
    pub l1b:  [[f32; L2]; NB_OUTPUT_BUCKETS],
    pub l2w: [[[f32; L3]; NB_OUTPUT_BUCKETS]; L2],
    pub l2b:  [[f32; L3]; NB_OUTPUT_BUCKETS],
    pub l3w:  [[f32; NB_OUTPUT_BUCKETS]; L3],
    pub l3b:   [f32; NB_OUTPUT_BUCKETS],
}

/// Weights and biases for the NNUE, quantized and embedded in the executable.
#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy)]
pub struct QuantNNUEData {
    pub ftw:   [i16; L1 * FEATURES * NB_INPUT_BUCKETS],
    pub ftb:   [i16; L1],
    pub l1w: [[[i8 ; L2]; NB_OUTPUT_BUCKETS]; L1],
    pub l1b:  [[f32; L2]; NB_OUTPUT_BUCKETS],
    pub l2w: [[[f32; L3]; NB_OUTPUT_BUCKETS]; L2],
    pub l2b:  [[f32; L3]; NB_OUTPUT_BUCKETS],
    pub l3w:  [[f32; NB_OUTPUT_BUCKETS]; L3],
    pub l3b:   [f32; NB_OUTPUT_BUCKETS],
}

/// Raw NNUE data.
pub static NNUE_EMBEDDED: QuantNNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("NNUE_EVALFILE"))) };

static PERMUTED_NNUE: OnceLock<Box<NNUEData>> = OnceLock::new();

/// Get the NNUE and permute it into a format for fast inference.
/// Only runs once and stores the result.
pub fn get_permuted_nnue() -> &'static NNUEData {
    // This funny business is here to make sure we never put the NNUE on the stack.
    PERMUTED_NNUE.get_or_init(|| unsafe {
        let mut nn = Box::<QuantNNUEData>::new_uninit();
        std::ptr::copy_nonoverlapping(&NNUE_EMBEDDED as *const QuantNNUEData, nn.as_mut_ptr(), 1);
        let nn = nn.assume_init();
        nn.permute()
    })
}
