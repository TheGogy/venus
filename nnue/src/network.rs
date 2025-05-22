use chess::types::{board::Board, eval::Eval};

use crate::accumulator::Accumulator;

/// The NNUE, as can be used in the position.
#[derive(Clone, Copy, Debug, Default)]
pub struct NNUE {
    network: Accumulator,
}

impl NNUE {
    /// Update the network and get the evaluation with the given board.
    pub fn update_and_evaluate(&mut self, b: &Board) -> Eval {
        self.network.update(b);
        Eval(self.network.propagate(b.stm))
    }
}
