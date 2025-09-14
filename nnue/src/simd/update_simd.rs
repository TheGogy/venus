use utils::memory::Align64;

use crate::{
    arch::L1,
    simd::{CHUNK_SIZE, vi16::*},
};

pub fn add1sub1_inplace(dst: &mut Align64<[i16; L1]>, sub0: &Align64<[i16; L1]>, add0: &Align64<[i16; L1]>) {
    for i in 0..L1 / CHUNK_SIZE {
        let vptr = unsafe { dst.0.as_mut_ptr().add(i * CHUNK_SIZE) };
        let s0ptr = unsafe { sub0.0.as_ptr().add(i * CHUNK_SIZE) };
        let a0ptr = unsafe { add0.0.as_ptr().add(i * CHUNK_SIZE) };

        let v = from_ptr_mut(vptr);
        let s0 = from_ptr(s0ptr);
        let a0 = from_ptr(a0ptr);

        to_ptr(add16(sub16(v, s0), a0), vptr);
    }
}

pub fn add1_inplace(dst: &mut Align64<[i16; L1]>, add0: &Align64<[i16; L1]>) {
    for i in 0..L1 / CHUNK_SIZE {
        let vptr = unsafe { dst.0.as_mut_ptr().add(i * CHUNK_SIZE) };
        let a0ptr = unsafe { add0.0.as_ptr().add(i * CHUNK_SIZE) };

        let v = from_ptr_mut(vptr);
        let a0 = from_ptr(a0ptr);

        to_ptr(add16(v, a0), vptr);
    }
}

pub fn sub1_inplace(dst: &mut Align64<[i16; L1]>, sub0: &Align64<[i16; L1]>) {
    for i in 0..L1 / CHUNK_SIZE {
        let vptr = unsafe { dst.0.as_mut_ptr().add(i * CHUNK_SIZE) };
        let s0ptr = unsafe { sub0.0.as_ptr().add(i * CHUNK_SIZE) };

        let v = from_ptr_mut(vptr);
        let s0 = from_ptr(s0ptr);

        to_ptr(sub16(v, s0), vptr);
    }
}
