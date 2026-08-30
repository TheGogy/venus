use chess::{
    defs::MAX_PLY,
    types::{board::Board, color::Color, eval::Eval, moves::Move},
};

use crate::{
    arch::{NNUEData, SCALE},
    embed::get_permuted_nnue,
    features::{
        Accumulator, output_bucket,
        psqt::{PsqtAccumulator, PsqtCache},
        update_stack,
    },
    inference::propagate::propagate_all_layers,
};

/// We will search up to [`MAX_PLY`] - so we need 1 extra accumulator to account for any moves made in
/// that final search.
const MAX_ACCS: usize = MAX_PLY + 1;

/// Allocate one accumulator per ply.
#[allow(clippy::unnecessary_box_returns)]
fn new_stack<A: Accumulator>(nn: &NNUEData) -> Box<[A; MAX_ACCS]> {
    let stack: Box<[A]> = (0..MAX_ACCS).map(|_| A::new(nn)).collect::<Vec<_>>().into_boxed_slice();
    let Ok(stack) = stack.try_into() else { unreachable!("Error allocating stack!!") };
    stack
}

/// NNUE.
/// This provides an interface for the neural network used to evaluate positions.
#[derive(Clone)]
pub struct NNUE {
    nn: &'static NNUEData,
    cache: PsqtCache,
    psqt_stack: Box<[PsqtAccumulator; MAX_ACCS]>,
    idx: usize,
}

impl Default for NNUE {
    #[allow(unreachable_code)]
    fn default() -> Self {
        #[cfg(not(feature = "embed"))]
        panic!("NNUE not embedded!!!!! Must use `embed` features and define EVALFILE");

        let nn = get_permuted_nnue();
        Self { cache: PsqtCache::new(nn), psqt_stack: new_stack(nn), idx: 0, nn }
    }
}

impl NNUE {
    /// Reset the NNUE.
    pub fn reset(&mut self) {
        self.cache.reset(self.nn);
        self.idx = 0;
    }

    /// A move has been made in the position: push its delta onto the stack.
    /// `b` must be the board *before* the move is made.
    pub fn move_made(&mut self, b: &Board, m: Move) {
        self.idx += 1;
        self.psqt_stack[self.idx].push_move(b, m);
    }

    /// A move has been undone in the position: pop 1 off the stack.
    pub const fn move_undo(&mut self) {
        self.idx -= 1;
    }

    /// Update everything in the NNUE to match the current board.
    /// This could be expensive, use refresh where possible.
    pub fn update_all(&mut self, b: &Board) {
        for pov in Color::iter() {
            self.psqt_stack[self.idx].refresh(self.nn, &mut self.cache, b, pov);
        }
    }

    /// Bring every input set at the current ply up to date with the board.
    fn update_incremental(&mut self, b: &Board) {
        update_stack(self.nn, &mut *self.psqt_stack, &mut self.cache, self.idx, b);
    }

    /// Evaluate the board using the NNUE.
    pub fn evaluate(&mut self, b: &Board) -> Eval {
        self.update_incremental(b);

        let obkt = output_bucket(b.occ().nbits() as usize);
        let acc = &self.psqt_stack[self.idx];
        debug_assert!(Color::iter().all(|pov| acc.correct(pov)));

        let (stm, opp) = match b.stm {
            Color::White => (&acc.values[0], &acc.values[1]),
            Color::Black => (&acc.values[1], &acc.values[0]),
        };

        let out = propagate_all_layers(self.nn, stm, opp, obkt);

        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        Eval((out * SCALE as f32) as i32)
    }
}
