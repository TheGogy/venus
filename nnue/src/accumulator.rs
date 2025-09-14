use chess::types::{bitboard::Bitboard, board::Board, color::Color, piece::Piece};
use utils::memory::Align64;

use crate::{
    arch::{L1, NNUE_EMBEDDED},
    simd::*,
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
    /// Update features and cache to match the current board.
    pub fn update(&mut self, b: &Board) {
        // King squares for each color.
        let wksq = b.ksq(Color::White);
        let bksq = b.ksq(Color::Black);

        // Update features.
        for c in 0..Color::NUM {
            let co = self.colors[c];
            let cn = b.colors[c];

            for p in 0..Piece::NUM {
                let old = co & self.pieces[p];
                let new = cn & b.pieces[p];

                let mut subs = old & !new;
                let mut adds = new & !old;

                // Handle both in one go if we can.
                while adds.any() && subs.any() {
                    let (wadd, badd) = NNUE_EMBEDDED.weights_for(c, p, wksq, bksq, adds.lsb());
                    let (wsub, bsub) = NNUE_EMBEDDED.weights_for(c, p, wksq, bksq, subs.lsb());

                    add1sub1_inplace(&mut self.w, wsub, wadd);
                    add1sub1_inplace(&mut self.b, bsub, badd);

                    adds.pop_lsb();
                    subs.pop_lsb();
                }

                // Toggle new weights on.
                adds.bitloop(|s| {
                    let (w, b) = NNUE_EMBEDDED.weights_for(c, p, wksq, bksq, s);
                    add1_inplace(&mut self.w, w);
                    add1_inplace(&mut self.b, b);
                });

                // Toggle old weights off.
                subs.bitloop(|s| {
                    let (w, b) = NNUE_EMBEDDED.weights_for(c, p, wksq, bksq, s);
                    sub1_inplace(&mut self.w, w);
                    sub1_inplace(&mut self.b, b);
                });
            }
        }

        // Update cache.
        self.colors = b.colors;
        self.pieces = b.pieces;
    }
}
