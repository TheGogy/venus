use chess::types::{eval::Eval, piece::Piece};

use crate::{
    maybe_const,
    tunables::params::tunables::{val_bishop, val_knight, val_pawn, val_queen, val_rook},
};

use super::pos::Pos;

/// Evaluation.
impl Pos {
    /// Very basic evaluation function, using this as a placeholder.
    pub fn evaluate(&self) -> Eval {
        maybe_const!(
            piece_vals: [i32; Piece::NUM] = [val_pawn(), val_knight(), val_bishop(), val_rook(), val_queen(), 0];
        );

        let stm = self.board.stm;
        let ntm = !stm;

        let mut total = 0;

        for p in Piece::iter() {
            total += (self.board.pc_bb(stm, p).nbits() - self.board.pc_bb(ntm, p).nbits()) as i32 * piece_vals[p.index()];
        }

        Eval(total)
    }
}
