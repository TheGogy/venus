use std::arch::x86_64::{
    __m128i, __m256i, _mm_loadu_si128, _mm256_adds_epu8, _mm256_and_si256, _mm256_blendv_epi8, _mm256_broadcastsi128_si256,
    _mm256_castsi256_si128, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_or_si256,
    _mm256_permute2x128_si256, _mm256_set1_epi8, _mm256_setzero_si256, _mm256_shuffle_epi8, _mm256_storeu_si256, _mm256_sub_epi8,
};

use chess::types::{board::Board, piece::CPiece, square::Square};
use utils::memory::Align64;

pub use crate::features::byteboard::scalar::Rays;
use crate::features::byteboard::{
    RayT, closest_occupied,
    luts::{INCOMING_SLIDERS_LUT, INCOMING_THREATS_LUT, NON_KNIGHTS_MASK, PERM_INDEX_LUT, PIECE_BIT_LUT},
    scalar::SLOT_IOTA,
};

pub type VecT = [__m256i; 2];

#[allow(clippy::cast_ptr_alignment)] // All loads and stores here are unaligned.
unsafe fn load(ptr: *const u8) -> VecT {
    unsafe { [_mm256_loadu_si256(ptr.cast()), _mm256_loadu_si256(ptr.add(32).cast())] }
}

pub fn splat_board(b: &Board) -> VecT {
    unsafe { load(b.pc_map.as_ptr().cast()) }
}

pub fn set_square(board: VecT, sq: Square, pc: CPiece) -> VecT {
    unsafe {
        let iota = load(SLOT_IOTA.as_ptr());
        let (sq, pc) = (_mm256_set1_epi8(sq as i8), _mm256_set1_epi8(pc as i8));
        [_mm256_blendv_epi8(board[0], pc, _mm256_cmpeq_epi8(iota[0], sq)), _mm256_blendv_epi8(board[1], pc, _mm256_cmpeq_epi8(iota[1], sq))]
    }
}

/// Look 16 bytes of a board up by `idx`. Indices outside `base..base + 16` are set to 0: the
/// saturating add pushes them past the sign bit, and are zeroed by `pshufb`.
unsafe fn lookup16(chunk: __m256i, idx: __m256i, base: i8) -> __m256i {
    unsafe {
        let sel = _mm256_adds_epu8(_mm256_sub_epi8(idx, _mm256_set1_epi8(base)), _mm256_set1_epi8(0x70));
        _mm256_shuffle_epi8(chunk, sel)
    }
}

/// Gather 32 board bytes by `idx`. Slots that fall off the board come back as `0xFF`, which
/// [`PIECE_BIT_LUT`] reads as empty.
unsafe fn gather(chunks: &[__m256i; 4], idx: __m256i) -> __m256i {
    unsafe {
        let lo = _mm256_or_si256(lookup16(chunks[0], idx, 0), lookup16(chunks[1], idx, 16));
        let hi = _mm256_or_si256(lookup16(chunks[2], idx, 32), lookup16(chunks[3], idx, 48));
        _mm256_or_si256(_mm256_or_si256(lo, hi), _mm256_cmpgt_epi8(idx, _mm256_set1_epi8(63)))
    }
}

/// One bit per byte of `v` that is not zero.
#[allow(clippy::cast_sign_loss)]
unsafe fn nonzero_mask(v: VecT) -> RayT {
    unsafe {
        let zero = _mm256_setzero_si256();
        let lo = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v[0], zero)) as u32;
        let hi = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v[1], zero)) as u32;
        !(u64::from(lo) | u64::from(hi) << 32)
    }
}

/// One bit per byte of `v` that shares a bit with `lut`.
unsafe fn test_mask(v: VecT, lut: &Align64<[u8; 64]>) -> RayT {
    unsafe {
        let lut = load(lut.as_ptr());
        nonzero_mask([_mm256_and_si256(v[0], lut[0]), _mm256_and_si256(v[1], lut[1])])
    }
}

impl Rays {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn new(board: VecT, sq: Square) -> Self {
        unsafe {
            let sqs = &PERM_INDEX_LUT[sq.idx()];
            let perm = load(sqs.as_ptr());

            let chunks = [
                _mm256_broadcastsi128_si256(_mm256_castsi256_si128(board[0])),
                _mm256_permute2x128_si256::<0x11>(board[0], board[0]),
                _mm256_broadcastsi128_si256(_mm256_castsi256_si128(board[1])),
                _mm256_permute2x128_si256::<0x11>(board[1], board[1]),
            ];
            let pcs = [gather(&chunks, perm[0]), gather(&chunks, perm[1])];

            let lut = _mm256_broadcastsi128_si256(_mm_loadu_si128(PIECE_BIT_LUT.as_ptr().cast::<__m128i>()));
            let bits = [_mm256_shuffle_epi8(lut, pcs[0]), _mm256_shuffle_epi8(lut, pcs[1])];

            let closest = closest_occupied(nonzero_mask(bits));
            let mut rays = Self {
                pcs: Align64([0; 64]),
                sqs: &sqs.0,
                closest,
                threats: test_mask(bits, &INCOMING_THREATS_LUT) & closest,
                sliders: test_mask(bits, &INCOMING_SLIDERS_LUT) & closest & NON_KNIGHTS_MASK,
            };

            let out = rays.pcs.as_mut_ptr();
            _mm256_storeu_si256(out.cast(), pcs[0]);
            _mm256_storeu_si256(out.add(32).cast(), pcs[1]);
            rays
        }
    }
}
