use chess::types::{board::Board, moves::Move};

use crate::threading::thread::Thread;

use super::{MovePicker, Stage, score_move::TAC_BAD};

impl<const QUIET: bool> MovePicker<QUIET> {
    /// Get the next best move and its score.
    pub fn next(&mut self, board: &Board, t: &Thread) -> Option<(Move, i32)> {
        match self.stage {
            // No more moves to process
            Stage::NoMoves => return None,

            // Try to find and return the transposition table move first
            Stage::TTMove => {
                let tt_move = self.tt_move;
                if let Some(m) = self.find_pred(self.idx_cur, self.moves.len(), |m| m == tt_move) {
                    self.idx_cur += 1;
                    return Some((m, i32::MAX));
                }
                self.stage = Stage::ScoreTacticals;
            }

            // Score all tactical moves based on MVV-LVA and SEE
            Stage::ScoreTacticals => {
                self.score_tacticals(board, t);
                self.stage = Stage::GoodTacticals;
            }

            // Return all tactical moves with positive SEE
            Stage::GoodTacticals => {
                if let Some((m, s)) = self.partial_sort(self.idx_quiets) {
                    return Some((m, s));
                }

                if !QUIET {
                    return None;
                }

                self.stage = Stage::ScoreQuiets;
            }

            // Assign history scores to quiet moves for move ordering
            Stage::ScoreQuiets => {
                self.score_quiets(board, t);
                self.stage = Stage::Quiets;
            }

            // Return quiet moves ordered by history score
            Stage::Quiets => {
                if !self.skip_quiets {
                    if let Some((m, s)) = self.partial_sort(self.idx_noisy_bad) {
                        return Some((m, s));
                    }
                }
                self.idx_cur = self.idx_noisy_bad;
                self.stage = Stage::BadTacticals;
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

        self.next(board, t)
    }
}
