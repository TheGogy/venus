use utils::memory::Align64;

use crate::arch::L1;

pub fn add1sub1_inplace(dst: &mut Align64<[i16; L1]>, sub0: &Align64<[i16; L1]>, add0: &Align64<[i16; L1]>) {
    for i in 0..L1 {
        dst[i] += add0[i] - sub0[i];
    }
}

pub fn add1_inplace(dst: &mut Align64<[i16; L1]>, add0: &Align64<[i16; L1]>) {
    for i in 0..L1 {
        dst[i] += add0[i];
    }
}

pub fn sub1_inplace(dst: &mut Align64<[i16; L1]>, sub0: &Align64<[i16; L1]>) {
    for i in 0..L1 {
        dst[i] -= sub0[i];
    }
}
