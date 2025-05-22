use chess::types::color::Color;

use crate::{NNUE_EMBEDDED, accumulator::Accumulator};

use super::{QA, QAB, SCALE, flatten::flatten};

impl Accumulator {
    // Propagate through the layers
    pub fn propagate(&self, c: Color) -> i32 {
        let (stm, opp) = match c {
            Color::White => (self.w, self.b),
            Color::Black => (self.b, self.w),
        };

        let weights = &NNUE_EMBEDDED.output_weights;
        let sum = flatten(&stm, &weights[0]) + flatten(&opp, &weights[1]);
        (sum / QA + NNUE_EMBEDDED.output_bias as i32) * SCALE / QAB
    }
}
