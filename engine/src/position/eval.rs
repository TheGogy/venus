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

#[cfg(test)]
mod tests {
    use chess::types::eval::Eval;

    use crate::{
        position::pos::Pos,
        tunables::params::tunables::{val_queen, val_rook},
    };

    #[test]
    fn test_eval() {
        let b = Pos::default();
        assert_eq!(b.evaluate(), Eval::DRAW);

        let b: Pos = "fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBN1 w Qkq - 0 1".parse().unwrap();
        assert_eq!(b.evaluate(), Eval(-val_rook()));

        let b: Pos = "fen rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBN1 w Qkq - 0 1".parse().unwrap();
        assert_eq!(b.evaluate(), Eval(-val_rook() + val_queen()));
    }
}
