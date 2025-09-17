use ::utils::memory::Align64;
use chess::types::{color::Color, square::Square};

pub mod propagate;

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

// Raw NNUE data.
pub static NNUE_EMBEDDED: NNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("NNUE_EVALFILE"))) };

impl NNUEData {
    pub fn weights_for(&self, c: usize, p: usize, wksq: Square, bksq: Square, s: Square) -> (&Align64<[i16; L1]>, &Align64<[i16; L1]>) {
        unsafe {
            let (w, b) = ft_idx(c, p, wksq, bksq, s);

            (
                &*NNUE_EMBEDDED.feature_weights.get_unchecked(w..w + L1).as_ptr().cast(),
                &*NNUE_EMBEDDED.feature_weights.get_unchecked(b..b + L1).as_ptr().cast(),
            )
        }
    }
}

#[rustfmt::skip]
pub const fn ft_idx(c: usize, p: usize, mut wksq: Square, mut bksq: Square, s: Square) -> (usize, usize) {
    let mut wflip = 0;
    let mut bflip = 56;
    if wksq.idx() % 8 > 3 {
        wksq = wksq.fliph();
        wflip ^= 7
    }
    if bksq.idx() % 8 > 3 {
        bksq = bksq.fliph();
        bflip ^= 7
    }

    let wbucket = input_bucket_idx(wksq, Color::White);
    let bbucket = input_bucket_idx(bksq, Color::Black);
    let s = s.idx();

    let w = wbucket * 768 +      c  * 384 + p * 64 + (s ^ wflip);
    let b = bbucket * 768 + (1 ^ c) * 384 + p * 64 + (s ^ bflip);

    (w * L1, b * L1)
}

/// Get the current input bucket to use.
pub const fn input_bucket_idx(ksq: Square, c: Color) -> usize {
    BUCKET_MAP[ksq.idx() ^ (56 * c.idx())]
}

/// Get the current output bucket to use.
pub const fn output_bucket_idx(nb_pieces: usize) -> usize {
    const DIV: usize = usize::div_ceil(32, NB_OUTPUT_BUCKETS);
    (nb_pieces - 2) / DIV
}
