use crate::tunables::params::tunables::*;

use super::Position;
use chess::types::{eval::Eval, piece::Piece};

/// Evaluation.
impl Position {
    /// Evaluates the position using the NNUE.
    pub fn evaluate(&mut self) -> Eval {
        let mut v = self.nnue.update_and_evaluate(&self.board);

        // Scale by the amount of material on the board.
        // This helps us to incentivise trading down when the positional value is worse, or keep
        // material on the board when we might be winning.
        v = (v * self.material_scale()) / 1024;

        v
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
