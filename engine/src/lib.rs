pub mod bench;
pub mod interface;
pub mod tunables;

mod history;
mod movepick;
mod position;
mod search;
mod threading;
mod time_management;
mod tt;

#[cfg(not(feature = "tune"))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "tune")]
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-tune");
