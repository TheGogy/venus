pub mod ftperm;
pub mod load_write;
pub mod permute;

use crate::arch::{EmbedNNUEData, NNUEData};

impl EmbedNNUEData {
    /// Take a quantized net all the way to the layout inference wants: first the feature transform
    /// permutation, then the SIMD repermutation.
    #[must_use]
    pub fn prepare_nnue(mut self: Box<Self>) -> Box<NNUEData> {
        self.ftperm();
        self.permute()
    }
}
