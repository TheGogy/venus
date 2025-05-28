use chess::types::{board::Board, moves::Move};

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

            // Generate and score noisies.
            MPStage::PvNoisyGen | MPStage::QsNoisyGen => {
                self.gen_score_noisies(b, t);
            }

            // Return all winning noisies.
            MPStage::PvNoisyWin | MPStage::QsNoisyAll => {
                if self.ml_noisy_win.non_empty() {
                    return Some(self.ml_noisy_win.next().0);
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
                if !self.skip_quiets && self.ml_quiet.non_empty() {
                    return Some(self.ml_quiet.next().0);
                }
            }

            // Return remaining noisies.
            MPStage::PvNoisyLoss => {
                if self.ml_noisy_loss.non_empty() {
                    return Some(self.ml_noisy_loss.next().0);
                }
            }

            // Generate and score evasions.
            MPStage::EvGen => {
                self.gen_score_evasions(b, t);
            }

            // Return all evasions.
            MPStage::EvAll => {
                if self.ml_quiet.non_empty() {
                    return Some(self.ml_quiet.next().0);
                }
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
