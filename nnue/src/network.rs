use chess::types::{board::Board, color::Color, eval::Eval};
use utils::memory::box_array;

use crate::{
    accumulator::Accumulator,
    arch::{NB_INPUT_BUCKETS, NNUE_EMBEDDED, input_bucket_idx, output_bucket_idx},
};

/// The NNUE, as can be used in the position.
#[derive(Clone, Debug)]
pub struct NNUE {
    table: Box<[[Accumulator; 2 * NB_INPUT_BUCKETS]; 2 * NB_INPUT_BUCKETS]>,
}

impl Default for NNUE {
    fn default() -> Self {
        let mut table: Box<[[Accumulator; 2 * NB_INPUT_BUCKETS]; 2 * NB_INPUT_BUCKETS]> = box_array();

        for x in table.iter_mut() {
            for y in x.iter_mut() {
                y.w = NNUE_EMBEDDED.feature_bias;
                y.b = NNUE_EMBEDDED.feature_bias;
            }
        }

        Self { table }
    }
}

impl NNUE {
    /// Update the network and get the evaluation with the given board.
    pub fn update_and_evaluate(&mut self, b: &Board) -> Eval {
        let wbkt = input_bucket_idx(b.ksq(Color::White), Color::White);
        let bbkt = input_bucket_idx(b.ksq(Color::Black), Color::Black);
        let obkt = output_bucket_idx(b.occ().nbits() as usize);

        let acc = &mut self.table[wbkt][bbkt];

        acc.update(b);
        Eval(acc.propagate(b.stm, obkt))
    }
}
