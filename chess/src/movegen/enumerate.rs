use crate::{
    tables::{
        leaping_piece::{king_atk, knight_atk, pawn_atk},
        sliding_piece::{bishop_atk, rook_atk},
    },
    types::{
        bitboard::Bitboard,
        board::Board,
        direction::Direction,
        moves::{Move, MoveFlag},
        piece::Piece,
        square::Square,
    },
};

use super::MgType;

/// Move generation functions.
impl Board {
    pub fn enumerate_moves<F, MG: MgType>(&self, mut receiver: F)
    where
        F: FnMut(Move),
    {
        match self.state.checkers.nbits() {
            // No checkers: enumerate all moves.
            0 => {
                self.enumerate_pawn::<F, MG, false>(&mut receiver);
                self.enumerate_knight::<F, MG, false>(&mut receiver);
                self.enumerate_diag::<F, MG, false>(&mut receiver);
                self.enumerate_orth::<F, MG, false>(&mut receiver);
                self.enumerate_king::<F, MG, false>(&mut receiver);

                if MG::QUIET {
                    self.enumerate_castling(&mut receiver);
                }
            }

            // 1 checker: enumerate all moves in the checkmask.
            1 => {
                self.enumerate_pawn::<F, MG, true>(&mut receiver);
                self.enumerate_knight::<F, MG, true>(&mut receiver);
                self.enumerate_diag::<F, MG, true>(&mut receiver);
                self.enumerate_orth::<F, MG, true>(&mut receiver);
                self.enumerate_king::<F, MG, true>(&mut receiver);
            }

            // 2 pieces checking king: we can't block both, we have to escape.
            2 => {
                self.enumerate_king::<F, MG, true>(&mut receiver);
            }

            // Cannot have more than 2 checkers in position.
            _ => unreachable!(),
        }
    }

    /// Add all moves from a given square.
    fn add_moves<F, MG: MgType, const CHECK: bool>(&self, from: Square, mut dest: Bitboard, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        if CHECK {
            dest &= self.state.checkmask;
        }

        if MG::NOISY {
            (dest & self.c_bb(!self.stm)).bitloop(|to| {
                receiver(Move::new(from, to, MoveFlag::Capture));
            });
        }

        if MG::QUIET {
            (dest & !self.occ()).bitloop(|to| {
                receiver(Move::new(from, to, MoveFlag::Normal));
            });
        }
    }

    /// Generate all quiet promotions.
    fn add_promos<F, MG: MgType>(&self, src: Square, dst: Square, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        if MG::NOISY {
            receiver(Move::new(src, dst, MoveFlag::PromoQ));
        }

        if MG::QUIET {
            receiver(Move::new(src, dst, MoveFlag::PromoN));
            receiver(Move::new(src, dst, MoveFlag::PromoR));
            receiver(Move::new(src, dst, MoveFlag::PromoB));
        }
    }

    /// Generate all capture promotions.
    /// This should only be called if we are enumerating noisy moves.
    fn add_cpromos<F>(&self, src: Square, dst: Square, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        receiver(Move::new(src, dst, MoveFlag::CPromoQ));
        receiver(Move::new(src, dst, MoveFlag::CPromoN));
        receiver(Move::new(src, dst, MoveFlag::CPromoR));
        receiver(Move::new(src, dst, MoveFlag::CPromoB));
    }

    /// Add all pawn moves.
    fn enumerate_pawn<F, MG: MgType, const CHECK: bool>(&self, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        let up = Direction::up(self.stm);
        let ul = Direction::ul(self.stm);
        let ur = Direction::ur(self.stm);

        let diag = self.state.pin_diag[self.stm.idx()];
        let orth = self.state.pin_orth[self.stm.idx()];
        let occ = self.occ();

        let opps = self.c_bb(!self.stm);

        let pawns = self.pc_bb(self.stm, Piece::Pawn);

        // Promotions.
        let promo = pawns & Bitboard::PR[self.stm.idx()] & !orth;
        if promo.any() {
            if MG::NOISY {
                // Promotions with capture.
                // We can move within pinmask as long as we stay within pinmask.
                let mut cl = ((promo & !diag).shift(ul) | (promo & diag).shift(ul) & diag) & opps;
                let mut cr = ((promo & !diag).shift(ur) | (promo & diag).shift(ur) & diag) & opps;

                if CHECK {
                    cl &= self.state.checkmask;
                    cr &= self.state.checkmask;
                }

                cl.bitloop(|s| {
                    self.add_cpromos(s.sub_dir(ul), s, receiver);
                });

                cr.bitloop(|s| {
                    self.add_cpromos(s.sub_dir(ur), s, receiver);
                });
            }

            // Promotions without capture.
            // As queen promotions should be searched first, we consider them noisy moves.
            let mut bb = (promo & !diag).shift(up) & !occ;

            if CHECK {
                bb &= self.state.checkmask;
            }

            bb.bitloop(|s| {
                self.add_promos::<F, MG>(s.sub_dir(up), s, receiver);
            });
        }

        // En passants.
        if MG::NOISY && self.state.epsq != Square::Invalid {
            let eprank = Bitboard::EP[self.stm.idx()];
            let epcap = self.state.epsq.sub_dir(up).bb();
            let mut epbb = pawn_atk(!self.stm, self.state.epsq) & pawns & !orth;

            // If we are in check, only add checkmask if it is not the ep piece putting us in
            // check.
            if CHECK && (self.state.checkers & epcap).is_empty() {
                epbb &= self.state.checkmask;
            }

            epbb.bitloop(|src| {
                // Our pawn is pinned but taking enemy pawn makes us leave pinmask.
                if (src.bb() & diag).any() && (self.state.epsq.bb() & diag).is_empty() {
                    return;
                }

                // Prune orthgonal pins.
                if (eprank & rook_atk(self.ksq(self.stm), occ ^ src.bb() ^ epcap) & self.orth_bb(!self.stm)).is_empty() {
                    receiver(Move::new(src, self.state.epsq, MoveFlag::EnPassant));
                }
            });
        }

        // Pushes.
        if MG::QUIET {
            let p = pawns & !promo & !diag;
            let mut singles = ((p & !orth).shift(up) | (p & orth).shift(up) & orth) & !occ;
            let mut doubles = (Bitboard::DP[self.stm.idx()] & singles).shift(up) & !occ;

            if CHECK {
                singles &= self.state.checkmask;
                doubles &= self.state.checkmask;
            }

            doubles.bitloop(|s| {
                receiver(Move::new(s.sub_dir(up).sub_dir(up), s, MoveFlag::DoublePush));
            });

            singles.bitloop(|s| {
                receiver(Move::new(s.sub_dir(up), s, MoveFlag::Normal));
            });
        }

        // Captures.
        if MG::NOISY {
            let p = pawns & !promo & !orth;
            let mut cl = ((p & !diag).shift(ul) | (p & diag).shift(ul) & diag) & opps;
            let mut cr = ((p & !diag).shift(ur) | (p & diag).shift(ur) & diag) & opps;

            if CHECK {
                cl &= self.state.checkmask;
                cr &= self.state.checkmask;
            }

            cl.bitloop(|s| {
                receiver(Move::new(s.sub_dir(ul), s, MoveFlag::Capture));
            });

            cr.bitloop(|s| {
                receiver(Move::new(s.sub_dir(ur), s, MoveFlag::Capture));
            });
        }
    }

    /// Add all castling moves.
    /// This should only be called when we are enumerating quiet moves
    /// and are not in check.
    fn enumerate_castling<F>(&self, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        let ksq = self.ksq(self.stm);
        let occ = self.occ();
        let atk = self.state.attacked;
        let orth = self.state.pin_orth[self.stm.idx()];
        let ks_pin = Square::G1.relative(self.stm).bb() & orth;
        let qs_pin = (Square::C1.relative(self.stm).bb() | Square::B1.relative(self.stm).bb()) & orth;

        // Kingside.
        if self.state.castling.has_ks(self.stm) && ks_pin.is_empty() && self.castlingmask.can_castle::<true>(ksq, self.stm, occ, atk) {
            receiver(Move::new(ksq, Square::G1.relative(self.stm), MoveFlag::Castling));
        }

        // Queenside.
        if self.state.castling.has_qs(self.stm) && qs_pin.is_empty() && self.castlingmask.can_castle::<false>(ksq, self.stm, occ, atk) {
            receiver(Move::new(ksq, Square::C1.relative(self.stm), MoveFlag::Castling));
        }
    }

    /// Add all king moves.
    fn enumerate_king<F, MG: MgType, const CHECK: bool>(&self, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        let ksq = self.ksq(self.stm);
        self.add_moves::<F, MG, false>(ksq, king_atk(ksq) & !self.state.attacked, receiver);
    }

    /// Add all knight moves.
    fn enumerate_knight<F, MG: MgType, const CHECK: bool>(&self, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        let diag = self.state.pin_diag[self.stm.idx()];
        let orth = self.state.pin_orth[self.stm.idx()];
        // Pinned knights can never move: they move diagonally and orthogonally at once.
        let knights = self.pc_bb(self.stm, Piece::Knight) & !(diag | orth);

        knights.bitloop(|s| {
            self.add_moves::<F, MG, CHECK>(s, knight_atk(s) & !self.c_bb(self.stm), receiver);
        });
    }

    /// Add all diagonal slider moves.
    fn enumerate_diag<F, MG: MgType, const CHECK: bool>(&self, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        let diag = self.state.pin_diag[self.stm.idx()];
        let orth = self.state.pin_orth[self.stm.idx()];

        let candidates = self.diag_bb(self.stm) & !orth;
        let occ = self.occ();
        let ok = !self.c_bb(self.stm);

        // Non pinned bishop + queen.
        (candidates & !diag).bitloop(|s| {
            self.add_moves::<F, MG, CHECK>(s, bishop_atk(s, occ) & ok, receiver);
        });

        // Pinned bishop + queen.
        (candidates & diag).bitloop(|s| {
            self.add_moves::<F, MG, CHECK>(s, bishop_atk(s, occ) & ok & diag, receiver);
        });
    }

    // Add all orthogonal slider moves.
    fn enumerate_orth<F, MG: MgType, const CHECK: bool>(&self, receiver: &mut F)
    where
        F: FnMut(Move),
    {
        let diag = self.state.pin_diag[self.stm.idx()];
        let orth = self.state.pin_orth[self.stm.idx()];
        let candidates = self.orth_bb(self.stm) & !diag;
        let occ = self.occ();
        let ok = !self.c_bb(self.stm);

        // Non pinned rook + queen.
        (candidates & !orth).bitloop(|s| {
            self.add_moves::<F, MG, CHECK>(s, rook_atk(s, occ) & ok, receiver);
        });

        // Pinned rook + queen.
        (candidates & orth).bitloop(|s| {
            self.add_moves::<F, MG, CHECK>(s, rook_atk(s, occ) & ok & orth, receiver);
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        movegen::{Noisy, Quiet},
        types::board::Board,
    };

    #[test]
    fn test_mg_types_exclusive() {
        const POSITIONS: &[&str] = &[
            "4k3/8/8/4p3/3P4/8/8/4K3 w - - 0 1",
            "4k3/8/8/8/3Pp3/8/8/4K3 b - d3 0 1",
            "3k2r1/5PP1/8/8/8/8/5pp1/3K3R w - - 0 1",
            "3k4/8/2q5/2b5/2r5/8/2P5/2R1K3 b - - 0 1",
            "5r1k/3b1q1p/1npb4/1p6/pPpP1N2/2P4B/2NBQ1P1/5R1K b - - 0 1",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N1P/1PP1QPP1/R4RK1 b - - 0 1",
            "2rq1r1k/bbp1npp1/p2p3p/1p6/3PP3/1B2NN2/PPQ2PPP/2R2RK1 w - - 0 1",
            "3Qb1k1/1r2ppb1/pN1n2q1/Pp1Pp1Pr/4P3/5P1p/4BBR1/2R4K w - - 0 1",
        ];

        // Noisy and quiet moves should be mutually exclusive.
        for fen in POSITIONS {
            let b: Board = fen.parse().unwrap();

            let mut qm = Vec::new();
            b.enumerate_moves::<_, Quiet>(|m| qm.push(m));
            b.enumerate_moves::<_, Noisy>(|m| {
                assert!(!qm.contains(&m));
            });
        }
    }
}
