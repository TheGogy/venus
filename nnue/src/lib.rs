#![warn(clippy::all, clippy::nursery, clippy::perf, clippy::pedantic)]

pub mod arch;
pub mod embed;
pub mod inference;
pub mod net;
pub mod preprocess;

mod simd;
mod utils;

pub const ARCH: &str = simd::simd::ARCH_NAME;
