use std::arch::x86_64::{
    __m128i, __m512i, _mm_loadu_si128, _mm_storeu_si128, _mm_unpackhi_epi16, _mm_unpacklo_epi8, _mm_unpacklo_epi16, _mm512_broadcast_i32x4,
    _mm512_castsi512_si128, _mm512_loadu_si512, _mm512_mask_blend_epi8, _mm512_mask_mov_epi8, _mm512_maskz_compress_epi8,
    _mm512_permutex2var_epi8, _mm512_set_epi8, _mm512_set1_epi8, _mm512_set1_epi16, _mm512_shuffle_epi8, _mm512_shuffle_i64x2,
    _mm512_storeu_si512, _mm512_test_epi8_mask,
};

use chess::types::{board::Board, piece::CPiece, square::Square};

use crate::features::{
    byteboard::{
        DISCOVERY_WIDTH, FOCUS_WIDTH, RayT, closest_occupied,
        luts::{INCOMING_SLIDERS_LUT, INCOMING_THREATS_LUT, NON_KNIGHTS_MASK, PERM_INDEX_LUT, PIECE_BIT_LUT},
    },
    threat::ThreatDeltaList,
};

pub type VecT = __m512i;

/// Piece byte used in ray slots that fall off the board.
const OFF_BOARD_PIECE: i8 = i8::MIN;

pub fn splat_board(b: &Board) -> VecT {
    unsafe { _mm512_loadu_si512(b.pc_map.as_ptr().cast()) }
}

pub fn set_square(board: VecT, sq: Square, pc: CPiece) -> VecT {
    unsafe { _mm512_mask_blend_epi8(sq.bb().0, board, _mm512_set1_epi8(pc as i8)) }
}

fn flip(v: VecT) -> VecT {
    unsafe { _mm512_shuffle_i64x2(v, v, 0b01_00_11_10) }
}

/// The board laid out along the 64 ray slots around one square.
pub struct Rays {
    pcs: VecT,
    sqs: VecT,

    /// The nearest piece in each direction.
    pub closest: RayT,
    /// Which of those pieces attack the square.
    pub threats: RayT,
    /// Which of those are sliders aimed at the square.
    pub sliders: RayT,
}

impl Rays {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn new(board: VecT, sq: Square) -> Self {
        unsafe {
            let sqs = _mm512_loadu_si512(PERM_INDEX_LUT[sq.idx()].as_ptr().cast());
            let lut = _mm512_broadcast_i32x4(_mm_loadu_si128(PIECE_BIT_LUT.as_ptr().cast::<__m128i>()));

            let pcs = _mm512_permutex2var_epi8(board, sqs, _mm512_set1_epi8(OFF_BOARD_PIECE));
            let bits = _mm512_shuffle_epi8(lut, pcs);

            let test = |lut: &[u8; 64]| _mm512_test_epi8_mask(bits, _mm512_loadu_si512(lut.as_ptr().cast()));
            let closest = closest_occupied(_mm512_test_epi8_mask(bits, bits));

            Self {
                pcs,
                sqs,
                closest,
                threats: test(&INCOMING_THREATS_LUT) & closest,
                sliders: test(&INCOMING_SLIDERS_LUT) & closest & NON_KNIGHTS_MASK,
            }
        }
    }

    /// Push every threat between `pc` on `sq` and the pieces picked out by `rays`.
    pub fn push_focus<const OUTGOING: bool>(&self, dst: &mut ThreatDeltaList, rays: RayT, pc: CPiece, sq: Square) {
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
            let other_pc = _mm512_maskz_compress_epi8(rays, self.pcs);
            let other_sq = _mm512_maskz_compress_epi8(rays, self.sqs);
            let other = _mm512_permutex2var_epi8(other_pc, pair2_shuffle, other_sq);

            let ptr = dst.as_mut_ptr().add(dst.len()).cast();
            _mm512_storeu_si512(ptr, _mm512_mask_mov_epi8(focus, mask, other));
            dst.set_len(dst.len() + rays.count_ones() as usize);
        }
    }

    /// Push every `sliders[i]` -> `victims[i]` threat. A victim sits in the slot opposite its slider.
    #[allow(clippy::cast_ptr_alignment)] // The stores are unaligned.
    pub fn push_discovery(&self, dst: &mut ThreatDeltaList, sliders: RayT, victims: RayT) {
        debug_assert_eq!(sliders.count_ones(), victims.count_ones());
        debug_assert!(dst.len() + DISCOVERY_WIDTH <= dst.capacity());
        unsafe {
            let slider_pc = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(sliders, self.pcs));
            let slider_sq = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(sliders, self.sqs));
            let victim_pc = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(victims, flip(self.pcs)));
            let victim_sq = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(victims, flip(self.sqs)));

            // Each delta is (attacker, src, victim, dst), so the halves interleave as pairs of bytes.
            let slider_pairs = _mm_unpacklo_epi8(slider_pc, slider_sq);
            let victim_pairs = _mm_unpacklo_epi8(victim_pc, victim_sq);

            let ptr = dst.as_mut_ptr().add(dst.len()).cast::<__m128i>();
            _mm_storeu_si128(ptr.add(0), _mm_unpacklo_epi16(slider_pairs, victim_pairs));
            _mm_storeu_si128(ptr.add(1), _mm_unpackhi_epi16(slider_pairs, victim_pairs));
            dst.set_len(dst.len() + sliders.count_ones() as usize);
        }
    }
}
