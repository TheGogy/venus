use crate::{
    movegen::Allmv,
    types::{
        board::Board,
        moves::{Move, MoveFlag},
        piece::{CPiece, Piece},
    },
};

impl Board {
    /// Whether the given move is legal in this position.
    #[rustfmt::skip]
    pub fn is_legal(&self, m: Move) -> bool {
        use Piece::*;

        let flag = m.flag();
        let src_piece = self.pc_at(m.src());
        let in_check = self.in_check();

        if src_piece == CPiece::None {
            return false
        }

        let mut found = false;
        macro_rules! check {
            ($f:ident $(, $check:expr)? ) => {
                if in_check {
                    self.$f::<_, Allmv, true>(&mut |mv| found |= mv == m);
                } else {
                    self.$f::<_, Allmv, false>(&mut |mv| found |= mv == m);
                }
            };
        }

        match src_piece.pt() {
            None   => unreachable!(),

            Pawn   => check!(enumerate_pawn),
            Knight => check!(enumerate_knight),
            Bishop => check!(enumerate_diag),
            Rook   => check!(enumerate_orth),

            Queen => {
                check!(enumerate_diag);
                check!(enumerate_orth);
            }

            King => {
                if flag == MoveFlag::Castling {
                    self.enumerate_castling(&mut |mv| found |= mv == m);
                } else {
                    check!(enumerate_king);
                }
            }
        }

        found
    }
}
