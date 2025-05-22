use chess::types::{eval::Eval, piece::Piece};
use nnue::network::NNUE;

use crate::{
    tunables::params::tunables::*,
};

use super::pos::Pos;

/// Evaluation.
impl Pos {
    /// Evaluates the position using the NNUE.
    pub fn evaluate(&self, nnue: &mut NNUE) -> Eval {
        let mut v = nnue.update_and_evaluate(&self.board);

        v = (v * self.material_scale()) / 1024;

        v
    }

    /// Get the material scale for the position.
    #[inline]
    #[rustfmt::skip]
    fn material_scale(&self) -> i32 {
        let total_material = 
            self.board.p_bb(Piece::Knight).nbits() as i32 * val_knight() +
            self.board.p_bb(Piece::Bishop).nbits() as i32 * val_bishop() +
            self.board.p_bb(Piece::Rook).nbits() as i32 * val_rook() +
            self.board.p_bb(Piece::Queen).nbits() as i32 * val_queen();

        mat_scale_base() + (total_material / 32)
    }
}
