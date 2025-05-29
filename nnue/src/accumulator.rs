use chess::types::{bitboard::Bitboard, board::Board, color::Color, piece::Piece};
use utils::Align64;

use crate::{
    NNUE_EMBEDDED,
    arch::{L1, utils::feature_idx},
};

/// Accumulator for each side.
pub type SideAccumulator = Align64<[i16; L1]>;

/// Accumulator.
/// Contains a SideAccumulator for each side to propagate through the network,
/// and a cached representation of the board from the last state we evaluated from.
#[derive(Clone, Copy, Debug)]
pub struct Accumulator {
    // Features.
    pub w: SideAccumulator,
    pub b: SideAccumulator,

    // Cache.
    colors: [Bitboard; Color::NUM],
    pieces: [Bitboard; Piece::NUM],
}

/// By default, set to feature bias.
impl Default for Accumulator {
    fn default() -> Self {
        Self {
            w: NNUE_EMBEDDED.feature_bias,
            b: NNUE_EMBEDDED.feature_bias,
            colors: [Bitboard::EMPTY; Color::NUM],
            pieces: [Bitboard::EMPTY; Piece::NUM],
        }
    }
}

impl Accumulator {
    /// Toggle the specific features on and off for the accumulator.
    /// Force not inlining to stop the function from going over the I-cache size -
    /// When fully expanded and optimized, it's bigger than the compiler thinks.
    #[inline(never)]
    fn toggle_features<const ON: bool>(&mut self, ft: (usize, usize)) {
        let update = |acc: &mut SideAccumulator, idx: usize| {
            debug_assert!(idx + L1 <= NNUE_EMBEDDED.feature_weights.len());

            // Enough bounds checking already rust, it works, we have the assertion!
            // This change increases nps from ~2.6m to ~4.4m. (69% increase, nice)
            // SAFETY: length of acc is L1, length of `idx..idx + L1` is L1, and all indices are in
            // range.
            unsafe {
                acc.iter_mut().zip(NNUE_EMBEDDED.feature_weights.get_unchecked(idx..idx + L1)).for_each(|(acc_val, &weight)| {
                    *acc_val += if ON { weight } else { -weight };
                });
            }
        };

        update(&mut self.w, ft.0);
        update(&mut self.b, ft.1);
    }

    /// Update features and cache to match the current board.
    pub fn update(&mut self, b: &Board) {
        // Update features.
        for c in 0..Color::NUM {
            let co = self.colors[c];
            let cn = b.colors[c];

            for p in 0..Piece::NUM {
                let old = co & self.pieces[p];
                let new = cn & b.pieces[p];

                // Toggle new weights on.
                (new & !old).bitloop(|s| {
                    self.toggle_features::<true>(feature_idx(c, p, s.idx()));
                });

                // Toggle old weights off.
                (old & !new).bitloop(|s| {
                    self.toggle_features::<false>(feature_idx(c, p, s.idx()));
                });
            }
        }

        // Update cache.
        self.colors = b.colors;
        self.pieces = b.pieces;
    }
}
