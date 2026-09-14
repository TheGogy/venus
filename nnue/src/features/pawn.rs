use arrayvec::ArrayVec;
use chess::types::{bitboard::Bitboard, board::Board, color::Color, direction::Direction, piece::Piece, rank_file::Rank, square::Square};
use utils::cfor;

use crate::features::{
    orient::Orient,
    threat::{MAX_ACTIVE_THREATS, MAX_DELTA_FEATURES},
};

/// Pawns can stand on ranks 2..=7.
const VALID_SQUARES: usize = 48;
const VALID_SQ_ZERO: Square = Square::A2;

/// Total distinct pawn positions.
const PAWN_IDS: usize = VALID_SQUARES * Color::NUM;

/// Total Pawn features.
pub const PAWN_FEATURES: usize = PAWN_IDS * (PAWN_IDS - 1) / 2;

/// Max deltas per move.
pub const MAX_PAWN_DELTAS_PER_MOVE: usize = 64;

/// Bands that a pawn standing on each square can "see".
const PP_BANDS: [Bitboard; Square::NUM] = {
    let mut bands = [Bitboard::EMPTY; Square::NUM];
    cfor!(let mut sq = 0; sq < 64; sq += 1; {
        let square = Square::from_raw(sq);
        let file_bb = square.file().bb();
        let band_bb = file_bb.0 | file_bb.shift(Direction::East).0 | file_bb.shift(Direction::West).0;
        let mask_bb = Rank::R1.bb().0 | Rank::R8.bb().0 | square.bb().0;
        bands[sq as usize] = Bitboard(band_bb & !mask_bb);
    });
    bands
};

/// Idx of a pawn of colour `c` standing on `sq`, as seen through `orient`.
#[allow(clippy::cast_possible_truncation)]
pub fn pawn_idx(orient: Orient, sq: Square, c: Color) -> u16 {
    let off = orient.color(c) * VALID_SQUARES;
    (off + usize::from(orient.sq(sq)) - VALID_SQ_ZERO.idx()) as u16
}

/// Feature for the pair of pawns `ix` and `iy`.
pub fn pp_idx(ix: u16, iy: u16) -> u16 {
    let hi = ix.max(iy);
    let lo = ix.min(iy);
    hi * (hi - 1) / 2 + lo
}

pub fn collect_pawn_indices(b: &Board, orient: Orient, indices: &mut ArrayVec<u16, MAX_ACTIVE_THREATS>) {
    let pawns = |c| b.pc_bb(c, Piece::Pawn);

    for (cx, cy) in [(Color::White, Color::White), (Color::White, Color::Black), (Color::Black, Color::Black)] {
        for x in pawns(cx) {
            // Only keep victims below attacker.
            let dedup = if cx == cy { Bitboard::below_mask(x) } else { Bitboard::FULL };
            let ix = pawn_idx(orient, x, cx);

            for y in pawns(cy) & PP_BANDS[x.idx()] & dedup {
                indices.push(pp_idx(ix, pawn_idx(orient, y, cy)));
            }
        }
    }
}

/// The pawns either side of a move.
#[derive(Copy, Clone, Eq, PartialEq, Default)]
pub struct PawnDeltas {
    pub before: [Bitboard; Color::NUM],
    pub after: [Bitboard; Color::NUM],
}

impl PawnDeltas {
    /// Collect the pair features this move toggles, as seen through `orient`.
    pub fn collect_indices(
        &self,
        orient: Orient,
        adds: &mut ArrayVec<u16, MAX_DELTA_FEATURES>,
        subs: &mut ArrayVec<u16, MAX_DELTA_FEATURES>,
    ) {
        // No change.
        if self.before == self.after {
            return;
        }

        #[cfg(target_feature = "avx512vbmi2")]
        unsafe {
            self.collect_indices_vbmi2(orient, adds, subs);
        }

        #[cfg(not(target_feature = "avx512vbmi2"))]
        self.collect_indices_scalar(orient, adds, subs);
    }

    #[cfg(any(test, not(target_feature = "avx512vbmi2")))]
    pub fn collect_indices_scalar(
        &self,
        orient: Orient,
        adds: &mut ArrayVec<u16, MAX_DELTA_FEATURES>,
        subs: &mut ArrayVec<u16, MAX_DELTA_FEATURES>,
    ) {
        // A pawn is only unchanged if the same colour still stands there, so a pawn taking a pawn
        // counts as a change on both sides.
        for (pawns, other, out) in [(self.after, self.before, &mut *adds), (self.before, self.after, &mut *subs)] {
            let mut remaining = pawns[0] | pawns[1];

            for cx in Color::iter() {
                for x in pawns[cx.idx()] & !other[cx.idx()] {
                    remaining.pop(x);

                    let ix = pawn_idx(orient, x, cx);
                    let partners = remaining & PP_BANDS[x.idx()];

                    for cy in Color::iter() {
                        for y in partners & pawns[cy.idx()] {
                            out.push(pp_idx(ix, pawn_idx(orient, y, cy)));
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_feature = "avx512vbmi2")]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    unsafe fn collect_indices_vbmi2(
        &self,
        orient: Orient,
        adds: &mut ArrayVec<u16, MAX_DELTA_FEATURES>,
        subs: &mut ArrayVec<u16, MAX_DELTA_FEATURES>,
    ) {
        #[allow(clippy::wildcard_imports)]
        use std::arch::x86_64::*;

        unsafe {
            #[rustfmt::skip]
            let iota = _mm512_set_epi8(
                63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 49, 48,
                47, 46, 45, 44, 43, 42, 41, 40, 39, 38, 37, 36, 35, 34, 33, 32,
                31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16,
                15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,
            );

            // Id a friendly pawn would have on each square.
            let ids_friendly =
                _mm512_sub_epi8(_mm512_xor_si512(iota, _mm512_set1_epi8(orient.sq as i8)), _mm512_set1_epi8(VALID_SQ_ZERO.idx() as i8));

            // Pair `ix` off against every partner square in `partners`.
            let pair_off = |out: &mut ArrayVec<u16, MAX_DELTA_FEATURES>, ix: u16, partners: u64, ids: __m512i| {
                let n = partners.count_ones() as usize;
                if n == 0 {
                    return;
                }

                debug_assert!(n <= 16 && n <= out.remaining_capacity());
                let pids = _mm256_cvtepu8_epi16(_mm512_castsi512_si128(_mm512_maskz_compress_epi8(partners, ids)));

                // pp_idx: hi * (hi - 1) / 2 + lo.
                let ix = _mm256_set1_epi16(ix as i16);
                let (hi, lo) = (_mm256_max_epu16(ix, pids), _mm256_min_epu16(ix, pids));
                let prod = _mm256_mullo_epi16(hi, _mm256_sub_epi16(hi, _mm256_set1_epi16(1)));
                let idxs = _mm256_add_epi16(_mm256_srli_epi16::<1>(prod), lo);

                let len = out.len();
                let keep = (1u32 << n) - 1;
                _mm512_mask_storeu_epi16(out.as_mut_ptr().add(len).cast(), keep, _mm512_castsi256_si512(idxs));
                out.set_len(len + n);
            };

            for (pawns, other, out) in [(self.after, self.before, &mut *adds), (self.before, self.after, &mut *subs)] {
                let enemy = pawns[(!orient.pov()).idx()];
                let ids = _mm512_mask_add_epi8(ids_friendly, enemy.0, ids_friendly, _mm512_set1_epi8(VALID_SQUARES as i8));

                let changed = (pawns[0] & !other[0]) | (pawns[1] & !other[1]);
                let mut remaining = pawns[0] | pawns[1];

                for x in changed {
                    remaining.pop(x);

                    let off = if enemy.has(x) { VALID_SQUARES as u16 } else { 0 };
                    let ix = off + (x.idx() as u16 ^ u16::from(orient.sq)) - VALID_SQ_ZERO.idx() as u16;

                    pair_off(out, ix, (remaining & PP_BANDS[x.idx()]).0, ids);
                }
            }
        }
    }
}
