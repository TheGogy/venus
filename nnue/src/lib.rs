#![warn(clippy::all, clippy::nursery, clippy::perf, clippy::pedantic)]
#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

pub mod arch;
pub mod embed;
pub mod inference;
pub mod net;
pub mod preprocess;

mod simd;
mod utils;

pub const ARCH: &str = simd::simd::ARCH_NAME;
