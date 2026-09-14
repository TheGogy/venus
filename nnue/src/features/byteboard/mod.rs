//! Threat updates over a byteboard. See: <https://87flowers.com/byteboard-attack-tables-1/>

mod luts;
pub mod updates;

#[cfg(all(target_feature = "avx512vbmi", target_feature = "avx512vbmi2"))]
#[path = "vbmi2.rs"]
mod backend;

#[cfg(all(target_arch = "x86_64", not(all(target_feature = "avx512vbmi", target_feature = "avx512vbmi2"))))]
#[path = "avx2.rs"]
mod backend;

#[cfg(target_arch = "aarch64")]
#[path = "neon.rs"]
mod backend;

/// Pieces are pulled out of the ray slots one at a time where there is no byte compress.
#[cfg(not(all(target_feature = "avx512vbmi", target_feature = "avx512vbmi2")))]
mod scalar;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("No byteboard backend available: only x86_64 and aarch64 are supported.");

use chess::types::piece::CPiece;

use crate::features::byteboard::luts::OUTGOING_THREATS_MASK;

/// One bit per ray slot.
pub type RayT = u64;

/// Total number of threats a piece can have focused on it at once.
pub const FOCUS_WIDTH: usize = 16;

/// Total number of discovered attacks a move can reveal at once.
pub const DISCOVERY_WIDTH: usize = 8;

/// Keep only the slot holding the nearest piece in each direction.
pub const fn closest_occupied(occ: RayT) -> RayT {
    let o = occ | 0x81_81_81_81_81_81_81_81;
    (o ^ (o - 0x03_03_03_03_03_03_03_03)) & occ
}

/// The slots in `closest` that `pc` attacks.
pub const fn outgoing_threats(pc: CPiece, closest: RayT) -> RayT {
    OUTGOING_THREATS_MASK[pc.idx()] & closest
}

/// Smear each non-empty direction byte of `x` out to all 8 of its bits.
pub const fn ray_fill(x: RayT) -> RayT {
    debug_assert!(x & 0x01_01_01_01_01_01_01_01 == 0);
    let x = (x + 0x7E_7E_7E_7E_7E_7E_7E_7E) & 0x80_80_80_80_80_80_80_80;
    x | (x - (x >> 7))
}
