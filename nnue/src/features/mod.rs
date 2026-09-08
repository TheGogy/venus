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

use crate::arch::{HalfAcc, NNUEData};

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
