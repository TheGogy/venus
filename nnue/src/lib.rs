pub mod arch;
pub mod net;
pub mod preprocess;

mod inference;
mod simd;

pub const ARCH: &str = simd::simd::ARCH_NAME;
