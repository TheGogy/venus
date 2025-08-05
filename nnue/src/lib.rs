pub mod network;

mod accumulator;
mod arch;
mod simd;

use arch::NNUEData;

// Raw NNUE data.
pub static NNUE_EMBEDDED: NNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("NNUE_EVALFILE"))) };
