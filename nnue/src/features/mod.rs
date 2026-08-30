pub mod psqt;
pub mod threat;

use chess::types::{board::Board, color::Color, moves::Move};

use crate::arch::NNUEData;

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

    /// Record the move that led into this ply, and mark both perspectives out of date.
    /// `b` must be the board BEFORE the move is made.
    fn push_move(&mut self, b: &Board, m: Move);

    /// Whether `pov` is up to date.
    fn correct(&self, pov: Color) -> bool;

    /// Whether the delta into this ply is one `pov` requires a full refresh of the board.
    fn requires_full_refresh(&self, pov: Color) -> bool;

    /// Bring `pov` up to date by applying this ply's delta on top of `prev`.
    fn apply_delta(&mut self, nn: &NNUEData, prev: &Self, pov: Color);

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
            for i in correct..idx {
                if let (prev, [curr, ..]) = stack.split_at_mut(i + 1) {
                    curr.apply_delta(nn, &prev[i], pov);
                }
            }
        } else {
            stack[idx].refresh(nn, cache, b, pov);
        }
    }
}
