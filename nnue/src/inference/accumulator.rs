use chess::types::{color::Color, square::Square};

use crate::{
    arch::{HalfAcc, L1_LEN, NNUEData},
    embed::get_permuted_nnue,
};

#[derive(Clone, Debug)]
pub struct FullAcc {
    pub feats: [HalfAcc; Color::NUM],
    pub correct: [bool; Color::NUM],
    pub ksqs: [Square; Color::NUM],
}

impl Default for FullAcc {
    fn default() -> Self {
        let bias = get_permuted_nnue().ftb;
        Self { feats: [bias; Color::NUM], correct: [false; Color::NUM], ksqs: [Square::Invalid; Color::NUM] }
    }
}

impl FullAcc {
    pub const fn from_nn(nn: &NNUEData) -> Self {
        Self { feats: [nn.ftb; Color::NUM], correct: [false; Color::NUM], ksqs: [Square::Invalid; Color::NUM] }
    }
}

pub fn add1_inplace(acc: &mut HalfAcc, add0: &HalfAcc) {
    for i in 0..L1_LEN {
        acc[i] += add0[i];
    }
}

pub fn sub1_inplace(acc: &mut HalfAcc, sub0: &HalfAcc) {
    for i in 0..L1_LEN {
        acc[i] -= sub0[i];
    }
}

pub fn add1sub1_inplace(acc: &mut HalfAcc, add0: &HalfAcc, sub0: &HalfAcc) {
    for i in 0..L1_LEN {
        acc[i] += add0[i] - sub0[i];
    }
}

pub fn add1sub1(dst: &mut HalfAcc, src: &HalfAcc, add0: &HalfAcc, sub0: &HalfAcc) {
    for i in 0..L1_LEN {
        dst[i] = src[i] + add0[i] - sub0[i];
    }
}

pub fn add1sub2(dst: &mut HalfAcc, src: &HalfAcc, add0: &HalfAcc, sub0: &HalfAcc, sub1: &HalfAcc) {
    for i in 0..L1_LEN {
        dst[i] = src[i] + add0[i] - sub0[i] - sub1[i];
    }
}

pub fn add2sub2(dst: &mut HalfAcc, src: &HalfAcc, add0: &HalfAcc, add1: &HalfAcc, sub0: &HalfAcc, sub1: &HalfAcc) {
    for i in 0..L1_LEN {
        dst[i] = src[i] + add0[i] + add1[i] - sub0[i] - sub1[i];
    }
}
