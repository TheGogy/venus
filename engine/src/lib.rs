mod history;
mod position;
mod search;
mod threading;
mod timeman;
mod tt;

pub mod bench;
pub mod interface;
pub mod tunables;

#[cfg(not(feature = "tune"))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "tune")]
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-tune");
