use chess::types::{color::Color, square::Square};
use utils::memory::Align64;

use crate::arch::{L1, NNUE_EMBEDDED};

/// Accumulator for each side.
pub type HalfAcc = Align64<[i16; L1]>;

#[derive(Clone, Debug)]
pub struct FullAcc {
    pub feats: [HalfAcc; Color::NUM],
    pub correct: [bool; Color::NUM],
    pub ksqs: [Square; Color::NUM],
}

impl Default for FullAcc {
    fn default() -> Self {
        FullAcc { feats: [NNUE_EMBEDDED.feature_bias; Color::NUM], correct: [false; Color::NUM], ksqs: [Square::Invalid; Color::NUM] }
    }
}

/// Haha auto vectorization go brrr

pub fn add1_inplace(acc: &mut HalfAcc, add0: &HalfAcc) {
    for i in 0..L1 {
        acc[i] += add0[i];
    }
}

pub fn sub1_inplace(acc: &mut HalfAcc, sub0: &HalfAcc) {
    for i in 0..L1 {
        acc[i] -= sub0[i];
    }
}

pub fn add1sub1_inplace(acc: &mut HalfAcc, add0: &HalfAcc, sub0: &HalfAcc) {
    for i in 0..L1 {
        acc[i] += add0[i] - sub0[i];
    }
}

pub fn add1sub1(dst: &mut HalfAcc, src: &HalfAcc, add0: &HalfAcc, sub0: &HalfAcc) {
    for i in 0..L1 {
        dst[i] = src[i] + add0[i] - sub0[i];
    }
}

pub fn add1sub2(dst: &mut HalfAcc, src: &HalfAcc, add0: &HalfAcc, sub0: &HalfAcc, sub1: &HalfAcc) {
    for i in 0..L1 {
        dst[i] = src[i] + add0[i] - sub0[i] - sub1[i];
    }
}

pub fn add2sub2(dst: &mut HalfAcc, src: &HalfAcc, add0: &HalfAcc, add1: &HalfAcc, sub0: &HalfAcc, sub1: &HalfAcc) {
    for i in 0..L1 {
        dst[i] = src[i] + add0[i] + add1[i] - sub0[i] - sub1[i];
    }
}
