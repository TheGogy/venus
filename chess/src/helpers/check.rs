use crate::{
    tables::{
        atk_by_type,
        sliding_piece::{between, bishop_atk, rook_atk},
    },
    types::{
        board::Board,
        moves::{Move, MoveFlag},
        piece::Piece,
    },
};

impl Board {
    /// Whether our move gives direct check (ignore any other forms of check).
    pub fn gives_direct_check(&self, m: Move) -> bool {
        self.king_line(self.get_piece(m.src()).pt()).has(m.dst())
    }

    /// Whether a move puts the opponent in check on the current board.
    pub fn gives_check(&self, m: Move) -> bool {
        let stm = self.stm;
        let opp = !self.stm;

        let opp_ksq = self.ksq(opp);
        let opp_kbb = opp_ksq.bb();

        let (src, dst) = (m.src(), m.dst());
        let (sbb, dbb) = (src.bb(), dst.bb());

        let occ = self.occ() ^ sbb;

        // Direct check.
        if (self.king_line(self.pc_at(src).pt()) & dbb).any() {
            return true;
        }

        // Discovered check.
        // If we are in line with the enemy king, check if there is a sliding piece giving check.
        if between(opp_ksq, src).any()
            && ((bishop_atk(opp_ksq, occ) & self.c_diag(stm)).any() || (rook_atk(opp_ksq, occ) & self.c_orth(stm)).any())
        {
            return true;
        }

        match m.flag() {
            // We have checked the normal en passant stuff,
            // we just need to see if it leads to discovered check.
            MoveFlag::EnPassant => {
                let epsq = dst.forward(opp).bb();
                let ep_occ = (occ ^ epsq) | dbb;

                (bishop_atk(opp_ksq, ep_occ) & self.c_diag(stm)).any() || (rook_atk(opp_ksq, ep_occ) & self.c_orth(stm)).any()
            }

            // See if the rook puts the king in check.
            MoveFlag::Castling => {
                let (_, rt) = self.castlingmask.rook_src_dst(dst);
                (self.king_line(Piece::Rook) & rt.bb()).any()
            }

            // See if the piece we are promoting to puts the king in check.
            f if f.is_promo() => (atk_by_type(f.get_promo(), dst, occ) & opp_kbb).any(),

            // We have done all the checks for other move types already: they do not give check.
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::board::Board;
    #[test]
    fn test_gives_check() {
        macro_rules! make_gives_check_tests {
            ($($fen:expr, [$(($mv:expr, $res:expr))*];)*) => {
                $(
                    let b: Board = $fen.parse().unwrap();
                    println!("{}", $fen);
                    $(
                        println!("{}", $mv);
                        let m = b.find_move($mv).unwrap();
                        assert_eq!(b.gives_check(m), $res);
                    )*
                )*
            };
        }

        make_gives_check_tests!(
            "8/8/8/3k4/8/2P3R1/3KP2Q/1B3N2 w - - 0 1", [("c3c4", true) ("e2e4", true) ("f1e3", true) ("b1a2", true) ("g3d3", true) ("h2g2", true) ("h2h5", true)];
            "8/5r2/5k2/8/8/3K4/8/8 b - - 0 1", [("f7d7", true) ("f7e7", false)];
            "8/3r4/3b1k2/8/8/3K4/8/8 b - - 0 1", [("d6e7", true) ("d7d8", false)];
            "8/3q4/3b1k2/8/8/3K4/8/8 b - - 0 1", [("d6e7", true) ("d7b5", true) ("d7d8", false)];
            "3r4/3n4/5k2/8/8/3K4/8/8 b - - 0 1", [("d7e5", true)];
            "8/8/8/1KRpP1k1/8/8/8/8 w - d6 0 1", [("e5d6", true) ("c5d5", false) ("e5e6", false)];
            "8/5k2/8/1K1pP3/8/1Q6/8/8 w - d6 0 1", [("e5d6", true) ("b5c6", false) ("e5e6", true)];
            "8/5k2/8/1K1pP3/8/1Q6/8/8 w - d6 0 1", [("e5d6", true) ("b5c6", false) ("e5e6", true)];
            "8/5k2/8/1K1pP3/2R5/1Q6/8/8 w - d6 0 1", [("e5d6", false) ("e5e6", true)];
            "8/3P1k2/8/8/8/8/3K4/8 w - - 0 1", [("d7d8n", true) ("d7d8q", false)];
            "8/8/8/8/8/8/8/R3K2k w Q - 0 1", [("e1c1", true)];
            "8/8/8/8/8/8/8/R3KP1k w Q - 0 1", [("e1c1", false)];
            "2rkr3/1b1pbppp/1p1q1n2/p1pPp1N1/PnP1P3/1QNB4/1P1BKPPP/3RR3 w - - 6 15", [("g5f7", true) ("g5e6", true) ("c3b5", false)];
        );
    }
}
