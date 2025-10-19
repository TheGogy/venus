use chess::{
    defs::MAX_PLY,
    types::{board::Board, color::Color, dirtypiece::DirtyPieces, eval::Eval},
};

use utils::memory::boxed_zeroed;

use crate::{
    accumulator::*,
    arch::{NNUE_EMBEDDED, dequantize, king_changed, output_bucket_idx},
    finny::FinnyTable,
    propagate::acc_l1,
};

/// We will search up to MAX_PLY - so we need 1 extra accumulator to account for any moves made in
/// that final search.
const MAX_ACCS: usize = MAX_PLY + 1;

/// NNUE.
/// This provides an interface for the neural network used to evaluate positions.
#[derive(Clone, Debug)]
pub struct NNUE {
    pub cache: FinnyTable,
    pub stack: Box<[FullAcc; MAX_ACCS]>,
    pub dp_stack: [DirtyPieces; MAX_ACCS],
    pub idx: usize,
}

impl Default for NNUE {
    fn default() -> Self {
        NNUE { cache: FinnyTable::default(), stack: boxed_zeroed(), dp_stack: [DirtyPieces::None; MAX_ACCS], idx: 0 }
    }
}

impl NNUE {
    /// A move has been made in the position: add dirty pieces to the stack.
    pub fn move_made(&mut self, b: &Board, dps: DirtyPieces) {
        self.idx += 1;
        self.dp_stack[self.idx] = dps;

        for c in Color::iter() {
            self.stack[self.idx].correct[c.idx()] = false;
            self.stack[self.idx].ksqs[c.idx()] = b.ksq(c);
        }
    }

    /// A move has been undone in the position: pop 1 off the stack.
    pub const fn move_undo(&mut self) {
        self.idx -= 1
    }

    /// Update everything in the NNUE to match the current board.
    /// This could be expensive, use refresh where possible.
    pub fn update_all(&mut self, b: &Board) {
        self.idx = 0;
        self.cache.refresh_to_pos(&mut self.stack[self.idx], b, Color::White);
        self.cache.refresh_to_pos(&mut self.stack[self.idx], b, Color::Black);

        for c in Color::iter() {
            self.stack[self.idx].correct[c.idx()] = true;
            self.stack[self.idx].ksqs[c.idx()] = b.ksq(c);
        }
    }

    /// Refresh the accumulator to match the current board by applying
    /// DirtyPieces.
    fn refresh(&mut self, b: &Board) {
        for c in Color::iter() {
            if self.stack[self.idx].correct[c.idx()] {
                continue;
            }

            let ksq = self.stack[self.idx].ksqs[c.idx()];
            let mut i = self.idx - 1;

            loop {
                // King has moved: we need a full refresh.
                if king_changed(ksq, self.stack[i].ksqs[c.idx()], c) {
                    self.cache.refresh_to_pos(&mut self.stack[self.idx], b, c);
                    break;
                }

                // Found most recently updated table: update from here.
                if self.stack[i].correct[c.idx()] {
                    while i < self.idx {
                        assert!(i < MAX_PLY);

                        let (left, right) = self.stack.split_at_mut(i + 1);

                        let prev = &left[i].feats[c.idx()];
                        let mut cur = &mut right[0].feats[c.idx()];

                        match self.dp_stack[i + 1] {
                            DirtyPieces::Add1Sub1((a0p, a0s), (s0p, s0s)) => {
                                let add0 = NNUE_EMBEDDED.feats_for(ksq, c, a0p.pt(), a0p.color(), a0s);
                                let sub0 = NNUE_EMBEDDED.feats_for(ksq, c, s0p.pt(), s0p.color(), s0s);
                                add1sub1(&mut cur, &prev, add0, sub0);
                            }

                            DirtyPieces::Add1Sub2((a0p, a0s), (s0p, s0s), (s1p, s1s)) => {
                                let add0 = NNUE_EMBEDDED.feats_for(ksq, c, a0p.pt(), a0p.color(), a0s);
                                let sub0 = NNUE_EMBEDDED.feats_for(ksq, c, s0p.pt(), s0p.color(), s0s);
                                let sub1 = NNUE_EMBEDDED.feats_for(ksq, c, s1p.pt(), s1p.color(), s1s);
                                add1sub2(&mut cur, &prev, add0, sub0, sub1);
                            }

                            DirtyPieces::Add2Sub2((a0p, a0s), (a1p, a1s), (s0p, s0s), (s1p, s1s)) => {
                                let add0 = NNUE_EMBEDDED.feats_for(ksq, c, a0p.pt(), a0p.color(), a0s);
                                let add1 = NNUE_EMBEDDED.feats_for(ksq, c, a1p.pt(), a1p.color(), a1s);
                                let sub0 = NNUE_EMBEDDED.feats_for(ksq, c, s0p.pt(), s0p.color(), s0s);
                                let sub1 = NNUE_EMBEDDED.feats_for(ksq, c, s1p.pt(), s1p.color(), s1s);
                                add2sub2(&mut cur, &prev, add0, add1, sub0, sub1);
                            }

                            DirtyPieces::None => unreachable!(),
                        }

                        right[0].correct[c.idx()] = true;
                        i += 1;
                    }
                    break;
                }

                i -= 1;
            }
        }
    }

    /// Evaluate the board using the NNUE.
    pub fn evaluate(&mut self, b: &Board) -> Eval {
        self.refresh(b);

        let obkt = output_bucket_idx(b.occ().nbits() as usize);
        let acc = &self.stack[self.idx];

        let (stm, opp) = match b.stm {
            Color::White => (&acc.feats[0], &acc.feats[1]),
            Color::Black => (&acc.feats[1], &acc.feats[0]),
        };

        let weights = &NNUE_EMBEDDED.output_weights;
        let sum = acc_l1(stm, &weights[obkt][0]) + acc_l1(opp, &weights[obkt][1]);

        dequantize(sum, obkt)
    }
}
