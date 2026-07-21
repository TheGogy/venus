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

            // For probcut, we also want to make sure the TT move has a SEE over the threshold.
            MPStage::PcTT => {
                self.stage = self.stage.next();
                if self.tt_move.flag().is_noisy() && b.see(self.tt_move, self.see_threshold) {
                    return Some(self.tt_move);
                }
            }

            // Generate and score noisies, and get ready to return noisy moves.
            MPStage::PvNoisyGen | MPStage::QsNoisyGen | MPStage::PcNoisyGen => {
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

            // Return all moves over the given SEE threshold.
            MPStage::PcNoisyAll => {
                if self.move_list.has_moves::<LEFT>() {
                    let m = self.move_list.select_upto::<LEFT>();
                    if b.see(m, self.see_threshold) {
                        return Some(m);
                    }
                }
            }

            // No more moves to play: end here.
            MPStage::PvEnd | MPStage::QsEnd | MPStage::EvEnd | MPStage::PcEnd => {
                return None;
            }
        }

        self.stage = self.stage.next();
        self.next(b, t)
    }
}
