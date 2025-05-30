use crate::{
    tables::{
        leaping_piece::{king_atk, knight_atk, pawn_atk},
        sliding_piece::{bishop_atk, rook_atk},
    },
    types::{
        bitboard::Bitboard,
        board::Board,
        color::Color,
        eval::Eval,
        moves::{Move, MoveFlag},
        piece::{CPiece, Piece},
        square::Square,
    },
};

const P: i32 = 200;
const N: i32 = 780;
const B: i32 = 820;
const R: i32 = 1300;
const Q: i32 = 2500;

/// Static exchange evaluation.
impl Board {
    /// Most Valuable Victim, Least Valuable Attacker.
    const MVVLVA: [i32; 12] = [P, P, N, N, B, B, R, R, Q, Q, 0, 0];

    /// Static Exchange evaluation (SEE).
    /// This determines if we win after all captures are made on a given square.
    pub fn see(&self, m: Move, threshold: Eval) -> bool {
        let (src, dst) = (m.src(), m.dst());
        let flag = m.flag();

        if flag == MoveFlag::Castling {
            return true;
        }

        // Get our piece that will be captured.
        let victim = if flag.is_promo() { CPiece::create(self.stm, flag.get_promo()) } else { self.get_piece(src) };

        // Get the value of the piece that we will use to capture.
        let mut move_val = if flag.is_cap() {
            if flag == MoveFlag::EnPassant { Self::MVVLVA[0] } else { Self::MVVLVA[self.pc_at(dst).idx()] }
        } else {
            0
        };

        if flag.is_promo() {
            move_val += Self::MVVLVA[victim.idx()] - Self::MVVLVA[0];
        }

        // Stop if opponent is winning.
        let mut balance = move_val - threshold;
        if balance < 0 {
            return false;
        }

        // If balance is in our favor, we can stop now.
        balance -= Self::MVVLVA[victim.idx()];
        if balance >= 0 {
            return true;
        }

        let diag_sliders = self.p_bb(Piece::Queen) | self.p_bb(Piece::Bishop);
        let orth_sliders = self.p_bb(Piece::Queen) | self.p_bb(Piece::Rook);

        let mut occ = self.occ();
        occ.pop_bit(src);
        occ.pop_bit(dst);

        if flag == MoveFlag::EnPassant {
            occ.pop_bit(self.state.epsq.forward(!self.stm));
        }

        let mut atk = self.attackers_to(dst, occ) & occ;
        let mut stm = !self.stm;

        loop {
            let own_atk = atk & self.c_bb(stm);

            // Exit when we run out of attackers.
            if own_atk.is_empty() {
                break;
            }

            // Get the least valuable attacker.
            let (p, s) = self.get_lva(stm, own_atk);
            occ.pop_bit(s);

            let pt = p.pt();
            if [Piece::Queen, Piece::Bishop, Piece::Pawn].contains(&pt) {
                atk |= bishop_atk(dst, occ) & diag_sliders;
            }
            if [Piece::Queen, Piece::Rook].contains(&pt) {
                atk |= rook_atk(dst, occ) & orth_sliders;
            }

            atk &= occ;

            stm = !stm;
            balance = -balance - 1 - Self::MVVLVA[p.idx()];
            if balance >= 0 {
                // If our final recapturing piece is a king, and the opponent has another attacker,
                // then a positive balance should mean a loss.
                if pt == Piece::King && atk & self.c_bb(stm) != Bitboard::EMPTY {
                    return self.stm == stm;
                }

                break;
            }
        }

        stm != self.stm
    }

    /// Returns a bitboard of all pieces that can attack the given square.
    #[rustfmt::skip]
    fn attackers_to(&self, s: Square, occ: Bitboard) -> Bitboard {
          self.pc_bb(Color::White, Piece::Pawn) & pawn_atk(Color::Black, s)
        | self.pc_bb(Color::Black, Piece::Pawn) & pawn_atk(Color::White, s)
        | (self.p_bb(Piece::Bishop) | self.p_bb(Piece::Queen)) & bishop_atk(s, occ)
        | (self.p_bb(Piece::Rook)   | self.p_bb(Piece::Queen)) & rook_atk(s, occ)
        | self.p_bb(Piece::Knight) & knight_atk(s)
        | self.p_bb(Piece::King) & king_atk(s)
    }

    /// Gets the least valuable attacker to a position.
    fn get_lva(&self, c: Color, atk: Bitboard) -> (CPiece, Square) {
        let my_occ = self.c_bb(c);

        for p in Piece::iter() {
            let s = atk & self.pc_bb(c, p) & my_occ;

            if s.any() {
                return (CPiece::create(c, p), s.lsb());
            }
        }

        // The attacking bitboard will always contain at least one piece.
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        helpers::see::*,
        types::{board::Board, eval::Eval},
    };

    #[test]
    fn test_see() {
        #[rustfmt::skip]
        const SEE_TESTS: &[(&str, &str, i32, bool)] = &[
            ("2k5/8/8/4p3/8/8/2K1R3/8 w - - 0 1", "e2e5", 0, true),
            ("3k4/8/8/4p3/3P4/8/8/5K2 w - - 0 1", "d4e5", P, true),
            ("3k4/8/5p2/4p3/3P4/8/8/5K2 w - - 0 1", "d4e5", P, false),
            ("8/3k4/2n2b2/8/3P4/8/3KN3/8 b - - 0 1", "c6d4", P, true),
            ("8/3k4/2n2b2/8/3P4/8/3KN3/8 b - - 0 1", "c6d4", N, false),
            ("3kr3/8/4q3/8/4P3/5P2/8/3K4 b - - 0 1", "e6e4", 0, false),
            ("3kr3/8/4q3/8/4P3/5P2/8/3K4 b - - 0 1", "e6e4", -Q, true),
            ("8/3k4/2n2b2/8/3P4/3K4/4N3/8 b - - 0 1", "c6d4", P, false),
            ("5k2/2P5/4b3/8/8/8/8/2R2K2 w - - 0 1", "c7c8q", 0, true),
            ("5k2/2P5/4b3/8/8/8/8/3R1K2 w - - 0 1", "c7c8q", 0, false),
            ("8/3k2b1/2n2b2/8/3P4/3K4/4N3/8 b - - 0 1", "c6d4", 0, true),
            ("3k4/8/2q5/2b5/2r5/8/2P5/2R1K3 b - - 0 1", "c4c2", 0, false),
            ("3k4/8/2q5/2b5/2r5/8/2P5/2R1K3 b - - 0 1", "c4c2", P - R, true),
            ("2k5/3n2b1/2nq4/4R3/5P2/3N1N2/8/5K2 b - - 0 1", "d6e5", 0, false),
            ("2k5/3n2b1/2nq4/4R3/5P2/3N1N2/8/5K2 b - - 0 1", "d6e5", R - Q + P, true),
            ("5r1k/3b1q1p/1npb4/1p6/pPpP1N2/2P4B/2NBQ1P1/5R1K b - - 0 1", "d6f4", 0, false),
            ("5r1k/3b1q1p/1npb4/1p6/pPpP1N2/2P4B/2NBQ1P1/5R1K b - - 0 1", "d6f4", -P, true),
        ];

        for (fen, mov, threshold, result) in SEE_TESTS {
            let b: Board = fen.parse().unwrap();
            let m = b.find_move(mov).unwrap();
            println!("{}", b.to_fen());
            assert_eq!(b.see(m, Eval(*threshold)), *result);
        }
    }
}
