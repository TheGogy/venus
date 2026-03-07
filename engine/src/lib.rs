#![warn(clippy::all, clippy::perf)]

pub mod bench;
pub mod interface;
pub mod position;
pub mod time_management;
pub mod tunables;

mod history;
mod movepick;
mod search;
mod threading;
mod tt;

mod tb;

#[cfg(not(feature = "tune"))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "tune")]
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-tune");
