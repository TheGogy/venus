use chess::types::{bitboard::Bitboard, board::Board, color::Color, piece::Piece};

use crate::{
    accumulator::*,
    arch::{NB_INPUT_BUCKETS, NNUE_EMBEDDED, input_bucket},
};

/// Finny entry.
/// Stores what the board looked like last refresh, so we only need to apply the difference.
#[derive(Clone, Default, Debug)]
pub struct FinnyEntry {
    pub acc: FullAcc,
    pub colors: [[Bitboard; Color::NUM]; Color::NUM],
    pub pieces: [[Bitboard; Piece::NUM]; Color::NUM],
}

/// Finny table.
/// Stores all the finny entries for each bucket.
/// We use 2 * input buckets because everything on the E-H files
/// is mirrored and we want to store those separately.
#[derive(Clone, Default, Debug)]
pub struct FinnyTable(pub Box<[FinnyEntry; 2 * NB_INPUT_BUCKETS]>);

impl FinnyTable {
    pub fn refresh_to_pos(&mut self, acc: &mut FullAcc, b: &Board, perspective: Color) {
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
                while adds.any() && subs.any() {
                    let add = NNUE_EMBEDDED.feats_for(ksq, perspective, p, c, adds.lsb());
                    let sub = NNUE_EMBEDDED.feats_for(ksq, perspective, p, c, subs.lsb());

                    add1sub1_inplace(feats, add, sub);

                    adds.pop_lsb();
                    subs.pop_lsb();
                }

                // Toggle new weights on.
                adds.bitloop(|s| {
                    let add = NNUE_EMBEDDED.feats_for(ksq, perspective, p, c, s);
                    add1_inplace(feats, add);
                });

                // Toggle old weights off.
                subs.bitloop(|s| {
                    let sub = NNUE_EMBEDDED.feats_for(ksq, perspective, p, c, s);
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
