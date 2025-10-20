use ::utils::memory::Align64;
use chess::types::{color::Color, eval::Eval, piece::Piece, square::Square};

use crate::{accumulator::HalfAcc, simd::vi16::CHUNK_SIZE_I16};

// Quantization factors.
pub const SCALE: i32 = 400;
pub const QA: i32 = 255;
pub const QB: i32 = 64;
pub const QAB: i32 = QA * QB;

// Number of features in the input layer.
pub const FEATURES: usize = Piece::NUM * Square::NUM * 2;

// Layer sizes.
pub const L1: usize = 2048;

const _: () = assert!(L1 % CHUNK_SIZE_I16 == 0);

// King bucket map.
// We only use the buckets on the A-D files, the E-H files are mirrored, but we want to store them
// differently in the finny table, so it is easier to store the bucket map like this.
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
    pub feature_weights: [Align64<[i16; L1]>; FEATURES * NB_INPUT_BUCKETS],
    pub feature_bias:     Align64<[i16; L1]>,
    pub output_weights: [[Align64<[i16; L1]>; 2]; NB_OUTPUT_BUCKETS],
    pub output_bias:              [i16; NB_OUTPUT_BUCKETS],
}

// Raw NNUE data.
pub static NNUE_EMBEDDED: NNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("NNUE_EVALFILE"))) };

impl NNUEData {
    /// Get the weights for the given feature.
    pub const fn feats_for(&self, mut ksq: Square, perspective: Color, p: Piece, c: Color, mut s: Square) -> &HalfAcc {
        const PIECE_STRIDE: usize = Square::NUM;
        const OPPONENT_STRIDE: usize = Square::NUM * Piece::NUM;
        const BUCKET_STRIDE: usize = Square::NUM * Piece::NUM * 2;

        if king_mirrored(ksq) {
            s = s.fliph();
            ksq = ksq.fliph();
        }

        let bucket = input_bucket(ksq, perspective);
        let opponent = c.idx() ^ perspective.idx();

        let idx = bucket * BUCKET_STRIDE + opponent * OPPONENT_STRIDE + p.idx() * PIECE_STRIDE + s.relative(perspective).idx();

        &NNUE_EMBEDDED.feature_weights[idx]
    }
}

// Whether or not the board should be mirrored.
pub const fn king_mirrored(ksq: Square) -> bool {
    ksq.file().idx() > 3
}

/// Get the current input bucket to use.
pub const fn input_bucket(ksq: Square, c: Color) -> usize {
    BUCKET_MAP[ksq.relative(c).idx()]
}

/// Get the current output bucket to use.
pub const fn output_bucket_idx(nb_pieces: usize) -> usize {
    const DIV: usize = usize::div_ceil(32, NB_OUTPUT_BUCKETS);
    let obkt = (nb_pieces - 2) / DIV;
    assert!(obkt < NB_OUTPUT_BUCKETS);
    obkt
}

/// Whether the king has changed position.
pub const fn king_changed(ks1: Square, ks2: Square, c: Color) -> bool {
    (king_mirrored(ks1) != king_mirrored(ks2)) || (input_bucket(ks1, c) != input_bucket(ks2, c))
}

/// Dequantize the output of the network and turn it to a useable evaluation.
pub const fn dequantize(sum: i32, obkt: usize) -> Eval {
    Eval((sum / QA + NNUE_EMBEDDED.output_bias[obkt] as i32) * SCALE / QAB)
}
