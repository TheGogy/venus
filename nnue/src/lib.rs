#![warn(clippy::all, clippy::nursery, clippy::perf)]

pub mod arch;
pub mod embed;
pub mod inference;
pub mod net;
pub mod preprocess;

mod simd;

pub const ARCH: &str = simd::simd::ARCH_NAME;
