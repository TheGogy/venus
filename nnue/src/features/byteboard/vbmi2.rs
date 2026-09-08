use std::arch::x86_64::{
    __m128i, __m512i, __mmask64, _mm_loadu_si128, _mm_storeu_si128, _mm_unpackhi_epi16, _mm_unpacklo_epi8, _mm_unpacklo_epi16,
    _mm512_broadcast_i32x4, _mm512_castsi512_si128, _mm512_loadu_si512, _mm512_mask_blend_epi8, _mm512_mask_mov_epi8,
    _mm512_maskz_compress_epi8, _mm512_permutex2var_epi8, _mm512_set_epi8, _mm512_set1_epi8, _mm512_set1_epi16, _mm512_shuffle_epi8,
    _mm512_shuffle_i64x2, _mm512_storeu_si512, _mm512_test_epi8_mask,
};

use chess::types::{board::Board, piece::CPiece, square::Square};

use crate::features::{
    byteboard::luts::{INCOMING_SLIDERS_LUT, INCOMING_THREATS_LUT, NON_KNIGHTS_MASK, OUTGOING_THREATS_MASK, PERM_INDEX_LUT, PIECE_BIT_LUT},
    threat::ThreatDeltaList,
};

pub type VecT = __m512i;
pub type RayT = __mmask64;

/// Total number of threats a piece can have focused on it at once.
pub const FOCUS_WIDTH: usize = 16;

/// Total number of discovered attacks a move can reveal at once.
pub const DISCOVERY_WIDTH: usize = 8;

/// Piece byte handed to ray slots that fall off the board. Any value with the high bit set does.
const OFF_BOARD_PIECE: i8 = i8::MIN;

pub fn splat_board(b: &Board) -> VecT {
    unsafe { _mm512_loadu_si512(b.pc_map.as_ptr().cast()) }
}

pub fn flip_board(b: VecT) -> VecT {
    unsafe { _mm512_shuffle_i64x2(b, b, 0b01_00_11_10) }
}

/// The square each of the 64 ray slots around `s` reads from.
pub fn perm_for(s: Square) -> VecT {
    unsafe { _mm512_loadu_si512(PERM_INDEX_LUT[s.idx()].as_ptr().cast()) }
}

/// Smear each non-empty direction byte of `x` out to all 8 of its bits.
pub const fn ray_fill(x: RayT) -> RayT {
    debug_assert!(x & 0x01_01_01_01_01_01_01_01 == 0);
    let x = (x + 0x7E_7E_7E_7E_7E_7E_7E_7E) & 0x80_80_80_80_80_80_80_80;
    x | (x - (x >> 7))
}

pub fn set_square(board: VecT, sq: Square, pc: CPiece) -> VecT {
    unsafe { _mm512_mask_blend_epi8(sq.bb().0, board, _mm512_set1_epi8(pc as i8)) }
}

/// Lay the board out along the 64 ray slots around a square, as pieces and as attack bits.
#[allow(clippy::cast_ptr_alignment)]
pub fn board_to_rays(perm: VecT, board: VecT) -> (VecT, VecT) {
    unsafe {
        let lut = _mm512_broadcast_i32x4(_mm_loadu_si128(PIECE_BIT_LUT.as_ptr().cast::<__m128i>()));
        let pcs = _mm512_permutex2var_epi8(board, perm, _mm512_set1_epi8(OFF_BOARD_PIECE));
        let bits = _mm512_shuffle_epi8(lut, pcs);
        (pcs, bits)
    }
}

pub fn closest_occupied(bits: VecT) -> RayT {
    let occ = unsafe { _mm512_test_epi8_mask(bits, bits) };
    let o = occ | 0x81_81_81_81_81_81_81_81;
    (o ^ (o - 0x03_03_03_03_03_03_03_03)) & occ
}

pub const fn outgoing_threats(pc: CPiece, closest: RayT) -> RayT {
    OUTGOING_THREATS_MASK[pc.idx()] & closest
}

pub fn incoming_threats(bits: VecT, closest: RayT) -> RayT {
    unsafe {
        let mask = _mm512_loadu_si512(INCOMING_THREATS_LUT.as_ptr().cast());
        _mm512_test_epi8_mask(bits, mask) & closest
    }
}

pub fn incoming_sliders(bits: VecT, closest: RayT) -> RayT {
    unsafe {
        let mask = _mm512_loadu_si512(INCOMING_SLIDERS_LUT.as_ptr().cast());
        _mm512_test_epi8_mask(bits, mask) & closest & NON_KNIGHTS_MASK
    }
}

/// Push every threat between `pc` on `sq` and the pieces picked out by `rays`.
pub fn push_focus<const OUTGOING: bool>(dst: &mut ThreatDeltaList, sqs: VecT, pcs: VecT, rays: RayT, pc: CPiece, sq: Square) {
    debug_assert!(dst.len() + FOCUS_WIDTH <= dst.capacity());
    unsafe {
        #[rustfmt::skip]
        let pair2_shuffle = _mm512_set_epi8(
            79, 15, 79, 15, 78, 14, 78, 14,
            77, 13, 77, 13, 76, 12, 76, 12,
            75, 11, 75, 11, 74, 10, 74, 10,
            73,  9, 73,  9, 72,  8, 72,  8,
            71,  7, 71,  7, 70,  6, 70,  6,
            69,  5, 69,  5, 68,  4, 68,  4,
            67,  3, 67,  3, 66,  2, 66,  2,
            65,  1, 65,  1, 64,  0, 64,  0,
        );
        let mask = if OUTGOING { 0xCCCC_CCCC_CCCC_CCCC } else { 0x3333_3333_3333_3333 };

        let focus = _mm512_set1_epi16(pc as i16 | (sq as i16) << 8);
        let other_pc = _mm512_maskz_compress_epi8(rays, pcs);
        let other_sq = _mm512_maskz_compress_epi8(rays, sqs);
        let other = _mm512_permutex2var_epi8(other_pc, pair2_shuffle, other_sq);

        let ptr = dst.as_mut_ptr().add(dst.len()).cast();
        _mm512_storeu_si512(ptr, _mm512_mask_mov_epi8(focus, mask, other));
        dst.set_len(dst.len() + rays.count_ones() as usize);
    }
}

/// Push every `sliders[i]` -> `victims[i]` threat.
#[allow(clippy::cast_ptr_alignment)] // The stores are unaligned.
pub fn push_discovery(dst: &mut ThreatDeltaList, sqs: VecT, pcs: VecT, sliders: RayT, victims: RayT) {
    debug_assert_eq!(sliders.count_ones(), victims.count_ones());
    debug_assert!(dst.len() + DISCOVERY_WIDTH <= dst.capacity());
    unsafe {
        let slider_pc = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(sliders, pcs));
        let slider_sq = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(sliders, sqs));
        let victim_pc = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(victims, flip_board(pcs)));
        let victim_sq = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(victims, flip_board(sqs)));

        // Each delta is (attacker, src, victim, dst), so the halves interleave as pairs of bytes.
        let slider_pairs = _mm_unpacklo_epi8(slider_pc, slider_sq);
        let victim_pairs = _mm_unpacklo_epi8(victim_pc, victim_sq);

        let ptr = dst.as_mut_ptr().add(dst.len()).cast::<__m128i>();
        _mm_storeu_si128(ptr.add(0), _mm_unpacklo_epi16(slider_pairs, victim_pairs));
        _mm_storeu_si128(ptr.add(1), _mm_unpackhi_epi16(slider_pairs, victim_pairs));
        dst.set_len(dst.len() + sliders.count_ones() as usize);
    }
}
