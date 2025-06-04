use chess::{
    MAX_MOVES,
    types::{board::Board, moves::Move},
};

use crate::threading::thread::Thread;

use super::{MPStage, MovePicker};

impl MovePicker {
    pub fn next(&mut self, b: &Board, t: &Thread) -> Option<Move> {
        match self.stage {
            // Return TT move.
            MPStage::PvTT | MPStage::QsTT | MPStage::EvTT => {
                self.stage = self.stage.next();
                return Some(self.tt_move);
            }

            // Generate and score noisies, and get ready to return noisy moves.
            MPStage::PvNoisyGen | MPStage::QsNoisyGen => {
                self.gen_score_noisies(b, t);
            }

            // Return all winning noisies.
            MPStage::PvNoisyWin | MPStage::QsNoisyAll | MPStage::EvAll => {
                if self.cur < self.left {
                    return Some(self.select_upto::<true>(self.left));
                }
            }

            // Generate and score quiets.
            MPStage::PvQuietGen => {
                if !self.skip_quiets {
                    self.gen_score_quiets(b, t);
                }
            }

            // Return all quiets.
            MPStage::PvQuietAll => {
                if !self.skip_quiets && self.cur < self.left {
                    return Some(self.select_upto::<true>(self.left));
                }

                // Go to the end and work backwards through the losing noisy moves.
                self.cur = MAX_MOVES - 1;
            }

            // Return all remaining moves.
            MPStage::PvNoisyLoss => {
                if self.cur > self.right {
                    return Some(self.select_upto::<false>(self.right));
                }
            }

            // Generate and score evasions.
            MPStage::EvGen => {
                self.gen_score_evasions(b, t);
            }

            // No more moves to play: end here.
            MPStage::PvEnd | MPStage::QsEnd | MPStage::EvEnd => {
                return None;
            }
        }

        self.stage = self.stage.next();
        self.next(b, t)
    }
}
