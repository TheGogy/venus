// Enable avx512 if available.
#![cfg_attr(all(target_arch = "x86_64", target_feature = "avx512f"), feature(stdarch_x86_avx512))]

pub mod network;

mod accumulator;
mod arch;
mod simd;

use arch::NNUEData;

// Raw NNUE data.
pub static NNUE_EMBEDDED: NNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("NNUE_EVALFILE"))) };
