use chess::types::{board::Board, moves::Move};

use crate::threading::thread::Thread;

use super::{MPStage, MovePicker};

impl<const QUIET: bool> MovePicker<QUIET> {
    pub fn next(&mut self, b: &Board, t: &Thread) -> Option<Move> {
        match self.stage {
            // No more moves
            MPStage::Finished => return None,

            // Score noisy moves.
            MPStage::NoisyScore => {
                self.score_noisies(b, &t.history);
                self.stage = MPStage::NoisyGood;
            }

            // Return good noisy moves.
            MPStage::NoisyGood => {
                if let Some(m) = self.partial_sort(self.idx_good_quiet) {
                    return Some(m);
                }

                if !QUIET {
                    return None;
                }

                self.stage = if QUIET { MPStage::QuietScore } else { MPStage::Finished }
            }

            // Score quiet moves.
            MPStage::QuietScore => {
                self.score_quiets(b, &t.history);
                self.stage = MPStage::QuietGood;
            }

            // Return good quiet moves.
            MPStage::QuietGood => {
                if let Some(m) = self.partial_sort(self.idx_bad_noisy) {
                    assert!(m.is_valid());
                    return Some(m);
                }
                self.stage = MPStage::NoisyBad;
                self.idx_cur = self.idx_bad_noisy;
            }

            MPStage::NoisyBad => {
                if let Some(m) = self.partial_sort(self.idx_end) {
                    return Some(m);
                }
                self.stage = MPStage::Finished;
            }
        }
        self.next(b, t)
    }
}
