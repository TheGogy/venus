use std::sync::OnceLock;

use crate::arch::{EmbedNNUEData, NNUEData};

/// Raw NNUE data.
#[cfg(all(feature = "embed", feature = "embed_direct"))]
pub static NNUE_EMBEDDED: NNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("EVALFILE"))) };

#[cfg(all(feature = "embed", not(feature = "embed_direct")))]
pub static NNUE_EMBEDDED: EmbedNNUEData = unsafe { std::mem::transmute(*include_bytes!(env!("EVALFILE"))) };

static PERMUTED_NNUE: OnceLock<Box<NNUEData>> = OnceLock::new();

#[allow(unused_mut)]
pub fn get_permuted_nnue() -> &'static NNUEData {
    PERMUTED_NNUE.get_or_init(|| unsafe {
        #[cfg(feature = "embed_direct")]
        {
            let mut nn = Box::<NNUEData>::new_uninit();
            #[cfg(feature = "embed")]
            std::ptr::copy_nonoverlapping(&raw const NNUE_EMBEDDED, nn.as_mut_ptr(), 1);
            nn.assume_init()
        }

        #[cfg(not(feature = "embed_direct"))]
        {
            let mut nn = Box::<EmbedNNUEData>::new_uninit();
            #[cfg(feature = "embed")]
            std::ptr::copy_nonoverlapping(&raw const NNUE_EMBEDDED, nn.as_mut_ptr(), 1);
            let nn = nn.assume_init();
            nn.prepare_nnue()
        }
    })
}
