use chess::types::{board::Board, moves::Move};

use crate::threading::thread::Thread;

use super::{
    MPStage, MovePicker,
    move_list::{LEFT, RIGHT},
};

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
                if self.move_list.has_moves::<LEFT>() {
                    return Some(self.move_list.select_upto::<LEFT>());
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
                if !self.skip_quiets && self.move_list.has_moves::<LEFT>() {
                    return Some(self.move_list.select_upto::<LEFT>());
                }

                // Get ready to go over losing noisy moves.
                self.move_list.prepare_bad_moves();
            }

            // Return all remaining moves.
            MPStage::PvNoisyLoss => {
                if self.move_list.has_moves::<RIGHT>() {
                    return Some(self.move_list.select_upto::<RIGHT>());
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
