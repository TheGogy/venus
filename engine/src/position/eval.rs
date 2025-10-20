use crate::{threading::thread::Thread, tunables::params::tunables::*};

use super::Position;
use chess::types::{eval::Eval, piece::Piece};

/// Evaluation.
impl Position {
    /// Evaluates the position using the NNUE.
    pub fn evaluate(&mut self) -> Eval {
        let mut v = self.nnue.evaluate(&self.board);

        // Scale by the amount of material on the board.
        // This helps us to incentivise trading down when the positional value is worse, or keep
        // material on the board when we might be winning.
        v = (v * self.material_scale()) / 1024;

        // Clamp eval to non-terminal range.
        v.clamped()
    }

    /// Adjust the evaluation according to correction history and 50 move rule scaling.
    pub fn adjust_eval(&mut self, t: &mut Thread, mut v: Eval) -> Eval {
        // Scale down the eval if we're just shuffling pieces back and forth and not making
        // progress.
        v = v * Eval(200 - self.board.state.halfmoves as i32) / Eval(200);

        // Add correction history.
        v += t.correction_score(&self.board);

        // Clamp eval to non-terminal range.
        v.clamped()
    }

    /// Get the material scale for the position.
    #[rustfmt::skip]
    fn material_scale(&self) -> i32 {
        let total_material =
            self.board.p_bb(Piece::Knight).nbits() as i32 * ms_knight() +
            self.board.p_bb(Piece::Bishop).nbits() as i32 * ms_bishop() +
            self.board.p_bb(Piece::Rook).nbits()   as i32 * ms_rook() +
            self.board.p_bb(Piece::Queen).nbits()  as i32 * ms_queen();

        ms_base() + (total_material / 32)
    }
}
