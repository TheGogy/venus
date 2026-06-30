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
    pub fn is_legal(&self, m: Move) -> bool {
        let flag = m.flag();
        let src_piece = self.pc_at(m.src());
        let in_check = self.in_check();

        if src_piece == CPiece::None {
            return false;
        }

        let mut found = false;
        macro_rules! search_moves {
            ($f:ident $(, $check:expr)? ) => {
                if in_check {
                    self.$f::<_, Allmv, true>(&mut |mv| found |= mv == m);
                } else {
                    self.$f::<_, Allmv, false>(&mut |mv| found |= mv == m);
                }
            };
        }

        match src_piece.pt() {
            Piece::None => unreachable!(),

            Piece::Pawn => search_moves!(enumerate_pawn),
            Piece::Knight => search_moves!(enumerate_knight),
            Piece::Bishop => search_moves!(enumerate_diag),
            Piece::Rook => search_moves!(enumerate_orth),

            Piece::Queen => {
                search_moves!(enumerate_diag);
                search_moves!(enumerate_orth);
            }

            Piece::King if flag == MoveFlag::Castling && !in_check => {
                self.enumerate_castling(&mut |mv| found |= mv == m);
            }
            Piece::King => {
                search_moves!(enumerate_king);
            }
        }

        found
    }
}
