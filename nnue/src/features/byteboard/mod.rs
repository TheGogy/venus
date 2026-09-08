//! See: <https://87flowers.com/byteboard-attack-tables-1/>

#[cfg(target_feature = "avx512vbmi2")]
mod vbmi2;

// #[cfg(all(target_feature = "avx2", not(target_feature = "avx512vbmi2")))]
// mod avx2;
// #[cfg(all(target_feature = "avx2", not(target_feature = "avx512vbmi2")))]
// use avx2::*;

mod luts;
pub mod updates;
