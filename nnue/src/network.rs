use chess::types::{board::Board, color::Color, eval::Eval};
use utils::box_array;

use crate::{
    NNUE_EMBEDDED,
    accumulator::Accumulator,
    arch::{BUCKET_NUM, utils::bucket_idx},
};

/// The NNUE, as can be used in the position.
#[derive(Clone, Debug)]
pub struct NNUE {
    table: Box<[[Accumulator; 2 * BUCKET_NUM]; 2 * BUCKET_NUM]>,
}

impl Default for NNUE {
    fn default() -> Self {
        let mut table: Box<[[Accumulator; 2 * BUCKET_NUM]; 2 * BUCKET_NUM]> = box_array();

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
        let wbkt = bucket_idx(b.ksq(Color::White), Color::White);
        let bbkt = bucket_idx(b.ksq(Color::Black), Color::Black);

        let acc = &mut self.table[wbkt][bbkt];

        acc.update(b);
        Eval(acc.propagate(b.stm))
    }
}
