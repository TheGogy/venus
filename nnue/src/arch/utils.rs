use chess::types::{color::Color, square::Square};

use crate::arch::NB_OUTPUT_BUCKETS;

use super::{BUCKET_MAP, L1};

/// Get the index of the features representing a given color and piece for white and black.
#[rustfmt::skip]
pub const fn ft_idx(c: usize, p: usize, mut wksq: Square, mut bksq: Square, s: Square) -> (usize, usize) {
    // If king is on right hand side of board, mirror.
    let mut wflip = 0;
    let mut bflip = 56;
    if wksq.idx() % 8 > 3 {
        wksq  = wksq.fliph();
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
