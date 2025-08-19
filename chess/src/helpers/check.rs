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
    /// Whether a move puts the opponent in check on the current board.
    pub fn gives_check(&self, m: Move) -> bool {
        let stm = self.stm;
        let opp = !self.stm;

        let opp_ksq = self.ksq(opp);
        let opp_kbb = opp_ksq.bb();

        let (src, dst) = (m.src(), m.dst());
        let (sbb, dbb) = (src.bb(), dst.bb());

        let occ = self.occ() ^ sbb;

        let pt = self.pc_at(src).pt();

        // Direct check.
        if (self.king_line(pt) & dbb).any() {
            return true;
        }

        // Discovered check.
        // If we are in line with the enemy king, check if there is a sliding piece giving check,
        // and that we have moved out of the way.
        if ((bishop_atk(opp_ksq, occ) & self.c_diag(stm)).any() || (rook_atk(opp_ksq, occ) & self.c_orth(stm)).any())
            && (between(opp_ksq, src) & between(opp_ksq, dst)).is_empty()
            && !(pt == Piece::Pawn && dst.forward(stm) == opp_ksq)
        {
            return true;
        }

        match m.flag() {
            // En passant.
            // We have checked the normal en passant stuff,
            // we just need to see if it leads to discovered check.
            MoveFlag::EnPassant => {
                let epsq = dst.forward(opp).bb();
                let ep_occ = (occ ^ epsq) | dbb;

                (bishop_atk(opp_ksq, ep_occ) & self.c_diag(stm)).any() || (rook_atk(opp_ksq, ep_occ) & self.c_orth(stm)).any()
            }

            // Castling.
            // This can give check if the enemy king is aligned with the square the rook will move
            // to, and there are no pieces between it and the enemy king.
            MoveFlag::Castling => {
                let (_, rt) = self.castlingmask.rook_src_dst(dst);
                let line = between(opp_ksq, rt);
                line.any() && (line & occ).is_empty()
            }

            // Promotions.
            // We have already checked the normal promotion stuff,
            // we just need to see if the piece we are promoting to puts the king in check.
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
            "8/8/8/8/8/8/8/R3K2k w Q - 0 1", [("e1c1", true) ("e1d1", false)];
            "8/8/8/8/8/8/8/R3KP1k w Q - 0 1", [("e1c1", false)];
            "2rkr3/1b1pbppp/1p1q1n2/p1pPp1N1/PnP1P3/1QNB4/1P1BKPPP/3RR3 w - - 6 15", [("g5f7", true) ("g5e6", true) ("c3b5", false)];
            "3k4/8/8/8/3P4/3R4/2K5/8 w - - 0 1", [("d4d5", false)];
            "7r/1kq1p1b1/1Pp2npp/2n5/8/2PbP2P/1Q3P2/rNB1KB2 w - - 0 1", [("b6c7", true)];
            "3N4/2p5/1p1pB1p1/1P2p1k1/p1P4r/P1q3K1/5R2/6R1 w - - 0 1", [("g3g2", false)];
            "3k4/3p4/8/8/8/8/8/R3K3 w Q - 0 1", [("e1d1", false)];
            "r6r/1k6/n1pq1n1p/P3p1p1/PB6/3bP2P/1Q3P2/RN1K1B2 b - - 0 1", [("d3f1", true)];
            "1n5r/1kq1p1b1/2p2n1p/1P3bp1/1Q6/2PPP2P/5P2/rNB1KB2 w - - 0 1", [("b5b6", false)];
        );
    }
}
