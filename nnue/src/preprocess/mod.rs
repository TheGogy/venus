pub mod ftperm;
pub mod load_write;
pub mod permute;
pub mod quantize;

use crate::arch::{NNUEData, QuantNNUEData};

impl QuantNNUEData {
    /// Take a quantized net all the way to the layout inference wants: first the feature transform
    /// permutation, then the SIMD repermutation.
    #[must_use]
    pub fn prepare_nnue(mut self: Box<Self>) -> Box<NNUEData> {
        self.ftperm();
        self.permute()
    }
}
