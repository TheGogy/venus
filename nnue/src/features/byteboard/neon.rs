use std::arch::aarch64::{
    uint8x16_t, uint8x16x4_t, vandq_u8, vbslq_u8, vceqq_u8, vdupq_n_u8, vgetq_lane_u64, vld1q_u8, vpaddq_u8, vqtbl1q_u8, vqtbx4q_u8,
    vreinterpretq_u64_u8, vst1q_u8, vtstq_u8,
};

use chess::types::{board::Board, piece::CPiece, square::Square};
use utils::memory::Align64;

pub use crate::features::byteboard::scalar::Rays;
use crate::features::byteboard::{
    RayT, closest_occupied,
    luts::{INCOMING_SLIDERS_LUT, INCOMING_THREATS_LUT, NON_KNIGHTS_MASK, PERM_INDEX_LUT, PIECE_BIT_LUT},
    scalar::SLOT_IOTA,
};

pub type VecT = [uint8x16_t; 4];

/// Piece byte used in ray slots that fall off the board: [`PIECE_BIT_LUT`] reads it as empty.
const OFF_BOARD_PIECE: u8 = 0xFF;

unsafe fn load(ptr: *const u8) -> VecT {
    unsafe { [vld1q_u8(ptr), vld1q_u8(ptr.add(16)), vld1q_u8(ptr.add(32)), vld1q_u8(ptr.add(48))] }
}

pub fn splat_board(b: &Board) -> VecT {
    unsafe { load(b.pc_map.as_ptr().cast()) }
}

pub fn set_square(board: VecT, sq: Square, pc: CPiece) -> VecT {
    unsafe {
        let iota = load(SLOT_IOTA.as_ptr());
        let (sq, pc) = (vdupq_n_u8(sq as u8), vdupq_n_u8(pc as u8));
        [
            vbslq_u8(vceqq_u8(iota[0], sq), pc, board[0]),
            vbslq_u8(vceqq_u8(iota[1], sq), pc, board[1]),
            vbslq_u8(vceqq_u8(iota[2], sq), pc, board[2]),
            vbslq_u8(vceqq_u8(iota[3], sq), pc, board[3]),
        ]
    }
}

/// Fold 64 all-ones / all-zeros bytes down to one bit each: weight every byte by its position in
/// its group of 8, then pairwise add three times.
unsafe fn pack_mask(v: VecT) -> RayT {
    const LANE_BITS: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128];
    unsafe {
        let w = vld1q_u8(LANE_BITS.as_ptr());
        let lo = vpaddq_u8(vandq_u8(v[0], w), vandq_u8(v[1], w));
        let hi = vpaddq_u8(vandq_u8(v[2], w), vandq_u8(v[3], w));
        let quads = vpaddq_u8(lo, hi);
        vgetq_lane_u64::<0>(vreinterpretq_u64_u8(vpaddq_u8(quads, quads)))
    }
}

/// One bit per byte of `v` that shares a bit with `lut`.
unsafe fn test_mask(v: VecT, lut: VecT) -> RayT {
    unsafe { pack_mask([vtstq_u8(v[0], lut[0]), vtstq_u8(v[1], lut[1]), vtstq_u8(v[2], lut[2]), vtstq_u8(v[3], lut[3])]) }
}

impl Rays {
    pub fn new(board: VecT, sq: Square) -> Self {
        unsafe {
            let sqs = &PERM_INDEX_LUT[sq.idx()];
            let perm = load(sqs.as_ptr());

            // Slots that index past the board keep the default byte.
            let tbl = uint8x16x4_t(board[0], board[1], board[2], board[3]);
            let off = vdupq_n_u8(OFF_BOARD_PIECE);
            let pcs = [
                vqtbx4q_u8(off, tbl, perm[0]),
                vqtbx4q_u8(off, tbl, perm[1]),
                vqtbx4q_u8(off, tbl, perm[2]),
                vqtbx4q_u8(off, tbl, perm[3]),
            ];

            let lut = vld1q_u8(PIECE_BIT_LUT.as_ptr());
            let bits = [vqtbl1q_u8(lut, pcs[0]), vqtbl1q_u8(lut, pcs[1]), vqtbl1q_u8(lut, pcs[2]), vqtbl1q_u8(lut, pcs[3])];

            let closest = closest_occupied(test_mask(bits, bits));
            let mut rays = Self {
                pcs: Align64([0; 64]),
                sqs: &sqs.0,
                closest,
                threats: test_mask(bits, load(INCOMING_THREATS_LUT.as_ptr())) & closest,
                sliders: test_mask(bits, load(INCOMING_SLIDERS_LUT.as_ptr())) & closest & NON_KNIGHTS_MASK,
            };

            let out = rays.pcs.as_mut_ptr();
            vst1q_u8(out, pcs[0]);
            vst1q_u8(out.add(16), pcs[1]);
            vst1q_u8(out.add(32), pcs[2]);
            vst1q_u8(out.add(48), pcs[3]);
            rays
        }
    }
}
