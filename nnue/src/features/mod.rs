/*
Features notes

We have 3 different input sources for the NNUE: PSQT inputs, Threat inputs, and Pawn inputs.

--- PSQT inputs ---

For PSQT inputs, features are encoded as (piece, square, stm ksq) tuples.
For 16 input experts, this gives us 2 * 6 * 64 * 16 = 12288 features.
All of these features are included in the network.

--- Threat inputs ---

For Threat inputs, features are encoded as (attacker, square, defender, square) tuples.
If we were to use all of that, it would be 2 * 6 * 64 * 2 * 6 * 64 = 589824 features.

Most of these features are redundant, we can reduce it by quite a bit:

1. Pieces can only attack certain squares, they will never threaten a square that is not in their pseudo-attack mask.
2. Features that imply other features are excluded (e.g Pawn -> Bishop is implied by Bishop -> Pawn).
3. If two pieces of the same type attack each other, one is redundant; Only keep if src > dst.
4. We don't call evaluation while in check: The King cannot be threatened.

We also have a separate input system for Pawn -> Pawn inputs, so those are also excluded.

Applying these, we get to 59808 features.

--- Pawn inputs ---

For Pawn inputs, features are encoded as (pawn a, pawn b) tuples.
Pawns can stand on ranks 2..=7, so there are 48 squares * 2 colors = 96 indices.

Since a pawn cannot pair with itself, this gives us 96 * 95 = 9120 features,
but since we only take the lower index we can actually halve this to 4560 features.

Pawn inputs are accumulated alongside the threat inputs. See [`ThreatAccumulator`].
*/

pub mod orient;
pub mod pawn;
pub mod psqt;
pub mod threat;

mod byteboard;

use chess::types::{board::Board, color::Color, moves::Move, square::Square};
use utils::min;

use crate::{
    arch::{HalfAcc, L1_LEN, NNUEData},
    simd,
};

/// Total number of output experts.
pub const NB_OUTPUT_BUCKETS: usize = 8;

/// Get the current output bucket to use.
pub const fn output_bucket(nb_pieces: usize) -> usize {
    const DIV: usize = usize::div_ceil(32, NB_OUTPUT_BUCKETS);
    (nb_pieces - 2) / DIV
}

/// One ply's worth of a single input set.
pub trait Accumulator: Sized {
    /// Whatever a full refresh needs besides the board.
    type Cache;

    /// Construct an accumulator from the given network.
    fn new(nn: &NNUEData) -> Self;

    /// Get the current values in the accumulator for the given perspective.
    fn values(&self, pov: Color) -> [&HalfAcc; Color::NUM];

    /// Record the move that led into this ply, and mark both perspectives out of date.
    /// `b` must be the board BEFORE the move is made.
    fn push_move_before(&mut self, b: &Board, m: Move);

    /// Record the position that move landed in.
    /// `b` must be the board AFTER the move is made.
    fn push_move_after(&mut self, b: &Board) {
        let _ = b;
    }

    /// Whether `pov` is up to date.
    fn correct(&self, pov: Color) -> bool;

    /// Whether the delta into this ply is one `pov` requires a full refresh of the board.
    fn requires_full_refresh(&self, pov: Color) -> bool;

    /// Bring `pov` up to date by applying this ply's delta on top of `prev`.
    fn apply_delta(&mut self, nn: &NNUEData, prev: &Self, pov: Color, ksq: Square);

    /// Rebuild `pov` from scratch to match the board `b`.
    fn refresh(&mut self, nn: &NNUEData, cache: &mut Self::Cache, b: &Board, pov: Color);
}

/// Bring `stack[idx]` up to date with `b`, for both perspectives.
pub fn update_stack<A: Accumulator>(nn: &NNUEData, stack: &mut [A], cache: &mut A::Cache, idx: usize, b: &Board) {
    for pov in Color::iter() {
        if stack[idx].correct(pov) {
            continue;
        }

        let mut correct_idx = None;
        for i in (0..idx).rev() {
            if stack[i + 1].requires_full_refresh(pov) {
                break;
            }
            if stack[i].correct(pov) {
                correct_idx = Some(i);
                break;
            }
        }

        if let Some(correct) = correct_idx {
            let ksq = b.ksq(pov);
            for i in correct..idx {
                if let (prev, [curr, ..]) = stack.split_at_mut(i + 1) {
                    curr.apply_delta(nn, &prev[i], pov, ksq);
                }
            }
        } else {
            stack[idx].refresh(nn, cache, b, pov);
        }
    }
}

/// One row of a feature transform.
pub(crate) trait FtRow: Copy {
    /// # Safety
    /// `ptr` must be valid for a read of [`simd::I16_LANES`] values, and vector aligned.
    unsafe fn load(ptr: *const Self) -> simd::I16Vec;
}

impl FtRow for i16 {
    unsafe fn load(ptr: *const Self) -> simd::I16Vec {
        unsafe { simd::load_i16(ptr) }
    }
}

impl FtRow for i8 {
    unsafe fn load(ptr: *const Self) -> simd::I16Vec {
        unsafe { simd::load_extend_i8(ptr) }
    }
}

/// Add every row in `adds` to `src` and take off every row in `subs`, writing the result to `dst`.
/// `REGS` accumulators are kept live at a time.
/// `FROM_ZERO` initializes sum as 0 instead of from `src`.
///
/// # Safety
/// `dst` must be valid for a write of [`L1_LEN`] i16s, every row must be valid for a read of
/// [`L1_LEN`] values, and `src` likewise unless `FROM_ZERO`. Everything must be vector aligned.
#[inline]
pub(crate) unsafe fn accumulate<T: FtRow, const REGS: usize, const FROM_ZERO: bool>(
    src: *const i16,
    dst: *mut i16,
    adds: &[*const T],
    subs: &[*const T],
) {
    // Fuse adds and subs where we can.
    let pairs = min!(adds.len(), subs.len());
    let mut regs = [simd::splat_i16(0); REGS];

    for base in (0..L1_LEN).step_by(REGS * simd::I16_LANES) {
        let off = |r: usize| base + r * simd::I16_LANES;

        unsafe {
            for (r, v) in regs.iter_mut().enumerate() {
                *v = if FROM_ZERO { simd::splat_i16(0) } else { simd::load_i16(src.add(off(r))) };
            }

            for i in 0..pairs {
                let (a, s) = (adds[i], subs[i]);
                for (r, v) in regs.iter_mut().enumerate() {
                    let o = off(r);
                    *v = simd::add_i16(*v, simd::sub_i16(T::load(a.add(o)), T::load(s.add(o))));
                }
            }
            for &a in &adds[pairs..] {
                for (r, v) in regs.iter_mut().enumerate() {
                    *v = simd::add_i16(*v, T::load(a.add(off(r))));
                }
            }
            for &s in &subs[pairs..] {
                for (r, v) in regs.iter_mut().enumerate() {
                    *v = simd::sub_i16(*v, T::load(s.add(off(r))));
                }
            }

            for (r, v) in regs.iter().enumerate() {
                simd::store_i16(dst.add(off(r)), *v);
            }
        }
    }
}
