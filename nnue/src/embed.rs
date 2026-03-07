use std::sync::OnceLock;

use crate::arch::{NNUEData, QuantNNUEData};

/// Raw NNUE data.
#[cfg(feature = "embed")]
pub static NNUE_EMBEDDED: QuantNNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("NNUE_EVALFILE"))) };

static PERMUTED_NNUE: OnceLock<Box<NNUEData>> = OnceLock::new();

/// Get the NNUE and permute it into a format for fast inference.
/// Only runs once and stores the result.
pub fn get_permuted_nnue() -> &'static NNUEData {
    // This funny business is here to make sure we never put the NNUE on the stack.
    PERMUTED_NNUE.get_or_init(|| unsafe {
        #[allow(unused_mut)]
        let mut nn = Box::<QuantNNUEData>::new_uninit();

        #[cfg(feature = "embed")]
        std::ptr::copy_nonoverlapping(&raw const NNUE_EMBEDDED, nn.as_mut_ptr(), 1);

        let nn = nn.assume_init();
        nn.permute()
    })
}
