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
        threat::ThreatAccumulator,
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
    thrt_stack: Box<[ThreatAccumulator; MAX_ACCS]>,
    idx: usize,
}

impl Default for NNUE {
    #[allow(unreachable_code)]
    fn default() -> Self {
        // #[cfg(not(feature = "embed"))]
        // panic!("NNUE not embedded!!!!! Must use `embed` features and define EVALFILE");

        Self::with_net(get_permuted_nnue())
    }
}

impl NNUE {
    /// Build an NNUE around an already permuted network.
    pub fn with_net(nn: &'static NNUEData) -> Self {
        Self { cache: PsqtCache::new(nn), psqt_stack: new_stack(nn), thrt_stack: new_stack(nn), idx: 0, nn }
    }

    /// Reset the NNUE.
    pub fn reset(&mut self) {
        self.cache.reset(self.nn);
        self.idx = 0;
    }

    /// Make a move on the board, pushing its delta onto the stack.
    pub fn make_move(&mut self, b: &mut Board, m: Move) {
        self.idx += 1;

        self.psqt_stack[self.idx].push_move_before(b, m);
        self.thrt_stack[self.idx].push_move_before(b, m);
        b.make_move(m);
        self.psqt_stack[self.idx].push_move_after(b);
        self.thrt_stack[self.idx].push_move_after(b);
    }

    /// Undo a move on the board, popping its delta off the stack.
    pub fn undo_move(&mut self, b: &mut Board) {
        b.undo_move();
        self.idx -= 1;
    }

    /// Update everything in the NNUE to match the current board.
    /// This could be expensive, use [`update_incremental`] where possible.
    pub fn update_all(&mut self, b: &Board) {
        for pov in Color::iter() {
            self.psqt_stack[self.idx].refresh(self.nn, &mut self.cache, b, pov);
            self.thrt_stack[self.idx].refresh(self.nn, &mut (), b, pov);
        }
    }

    /// Bring every input set at the current ply up to date with the board.
    fn update_incremental(&mut self, b: &Board) {
        update_stack(self.nn, &mut *self.psqt_stack, &mut self.cache, self.idx, b);
        update_stack(self.nn, &mut *self.thrt_stack, &mut (), self.idx, b);
    }

    /// Evaluate the board using the NNUE.
    pub fn evaluate(&mut self, b: &Board) -> Eval {
        #[allow(clippy::cast_possible_truncation)]
        Eval(self.evaluate_raw(b) as i32)
    }

    /// Evaluate the board using the NNUE, without rounding to whole centipawns.
    ///
    /// This is what the trainer prints for a position, so it is the value the offline tooling
    /// compares against.
    pub fn evaluate_raw(&mut self, b: &Board) -> f32 {
        self.update_incremental(b);

        let obkt = output_bucket(b.occ().nbits() as usize);
        let psqt_acc = &self.psqt_stack[self.idx];
        let thrt_acc = &self.thrt_stack[self.idx];

        debug_assert!(Color::iter().all(|pov| psqt_acc.correct(pov)));
        debug_assert!(Color::iter().all(|pov| thrt_acc.correct(pov)));

        propagate_all_layers(self.nn, psqt_acc.values(b.stm), thrt_acc.values(b.stm), obkt) * SCALE
    }
}
