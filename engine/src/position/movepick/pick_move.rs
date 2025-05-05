use chess::types::{board::Board, moves::Move};

use crate::threading::thread::Thread;

use super::{MovePicker, Stage, score_move::TAC_BAD};

impl<const QUIET: bool> MovePicker<QUIET> {
    /// Get the next best move and its score.
    pub fn next(&mut self, board: &Board, thread: &Thread) -> Option<(Move, i32)> {
        match self.stage {
            // No more moves to process
            Stage::NoMoves => return None,

            // Try to find and return the transposition table move first
            Stage::TTMove => {
                self.stage = Stage::ScoreTacticals;
                let tt_move = self.tt_move.unwrap();

                if let Some(m) = self.find_pred(self.idx_cur, self.moves.len(), |m| m == tt_move) {
                    self.idx_cur += 1;
                    return Some((m, i32::MAX));
                }
            }

            // Score all tactical moves based on MVV-LVA and SEE
            Stage::ScoreTacticals => {
                self.stage = Stage::GoodTacticals;
                self.score_tacticals(board, thread);
            }

            // Return all tactical moves with positive SEE
            Stage::GoodTacticals => {
                if let Some((m, s)) = self.partial_sort(self.idx_quiets) {
                    return Some((m, s));
                }

                if !QUIET {
                    return None;
                }

                self.stage = if QUIET { Stage::ScoreQuiets } else { Stage::NoMoves };
            }

            // Assign history scores to quiet moves for move ordering
            Stage::ScoreQuiets => {
                self.stage = Stage::Quiets;
                self.score_quiets(board, thread);
            }

            // Return quiet moves ordered by history score
            Stage::Quiets => {
                if !self.skip_quiets {
                    if let Some((m, s)) = self.partial_sort(self.idx_noisy_bad) {
                        return Some((m, s));
                    }
                }
                self.stage = Stage::BadTacticals;
                self.idx_cur = self.idx_noisy_bad;
            }

            // Return tactical moves with negative SEE (likely losing captures)
            Stage::BadTacticals => {
                if let Some((m, s)) = self.partial_sort(self.moves.len()) {
                    if !(self.skip_quiets && s == TAC_BAD) {
                        return Some((m, s));
                    }
                }

                // No more moves to process
                self.stage = Stage::NoMoves;
            }
        }

        self.next(board, thread)
    }
}
