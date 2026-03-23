use chess::types::{bitboard::Bitboard, board::Board, color::Color, piece::Piece};

use crate::{
    arch::{INPUT_KING_POSNS, NNUEData},
    inference::{
        accumulator::{FullAcc, add1_inplace, add1sub1_inplace, sub1_inplace},
        features::input_bucket,
    },
};

/// Finny entry.
/// Stores what the board looked like last refresh, so we only need to apply the difference.
#[derive(Clone, Default, Debug)]
pub struct FinnyEntry {
    pub acc: FullAcc,
    pub colors: [[Bitboard; Color::NUM]; Color::NUM],
    pub pieces: [[Bitboard; Piece::NUM]; Color::NUM],
}

impl FinnyEntry {
    pub const fn from_nn(nn: &NNUEData) -> Self {
        Self {
            acc: FullAcc::from_nn(nn),
            colors: [[Bitboard::EMPTY; Color::NUM]; Color::NUM],
            pieces: [[Bitboard::EMPTY; Piece::NUM]; Color::NUM],
        }
    }
}

/// Finny table.
/// Stores all the finny entries for each king input position.
#[derive(Clone, Debug)]
pub struct FinnyTable(pub Box<[FinnyEntry; INPUT_KING_POSNS]>);

impl FinnyTable {
    pub fn reset(&mut self) {
        for e in self.0.iter_mut() {
            *e = FinnyEntry::default();
        }
    }

    /// Get a [`FinnyTable`] from some given NNUE weights.
    pub fn from_nn(nn: &NNUEData) -> Self {
        let arr = (0..INPUT_KING_POSNS).map(|_| FinnyEntry::from_nn(nn)).collect::<Vec<_>>().into_boxed_slice().try_into().unwrap();
        Self(arr)
    }

    /// Fully refresh the entry to the given board, and update the accumulator.
    /// Use incremental updates wherever possible.
    pub fn refresh_to_pos(&mut self, nn: &NNUEData, acc: &mut FullAcc, b: &Board, perspective: Color) {
        let ksq = b.ksq(perspective);
        let entry = &mut self.0[input_bucket(ksq, perspective)];
        let feats = &mut entry.acc.feats[perspective.idx()];

        for c in Color::iter() {
            for p in Piece::iter() {
                let old = entry.pieces[perspective.idx()][p.idx()] & entry.colors[perspective.idx()][c.idx()];
                let new = b.pc_bb(c, p);

                let mut subs = old & !new;
                let mut adds = new & !old;

                // Handle both in one go if we can.
                while adds.non_empty() && subs.non_empty() {
                    let add = nn.feats_for(ksq, perspective, p, c, adds.lsb());
                    let sub = nn.feats_for(ksq, perspective, p, c, subs.lsb());

                    add1sub1_inplace(feats, add, sub);

                    adds.pop_lsb();
                    subs.pop_lsb();
                }

                // Toggle new weights on.
                adds.bitloop(|s| {
                    let add = nn.feats_for(ksq, perspective, p, c, s);
                    add1_inplace(feats, add);
                });

                // Toggle old weights off.
                subs.bitloop(|s| {
                    let sub = nn.feats_for(ksq, perspective, p, c, s);
                    sub1_inplace(feats, sub);
                });
            }
        }

        acc.correct[perspective.idx()] = true;
        acc.feats[perspective.idx()] = *feats;
        entry.colors[perspective.idx()] = b.colors;
        entry.pieces[perspective.idx()] = b.pieces;
    }
}
