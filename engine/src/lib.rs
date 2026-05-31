#![warn(clippy::all, clippy::perf)]

pub mod bench;
pub mod history;
pub mod interface;
pub mod movepick;
pub mod position;
pub mod search;
pub mod tb;
pub mod threading;
pub mod time_management;
pub mod tt;
pub mod tunables;

#[cfg(not(feature = "tune"))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "tune")]
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-tune");
