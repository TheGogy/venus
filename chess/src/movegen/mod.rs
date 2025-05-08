pub mod make_move;
pub mod perft;
pub mod update;

use crate::{
    tables::{
        leaping_piece::{king_atk, knight_atk, pawn_atk},
        sliding_piece::{bishop_atk, rook_atk},
    },
    types::{
        bitboard::Bitboard,
        board::Board,
        direction::Direction,
        move_list::MoveList,
        moves::{Move, MoveFlag},
        piece::Piece,
        square::Square,
    },
};

pub const ALL_MOVE: bool = true;
pub const TAC_ONLY: bool = false;

/// Move generation functions.
impl Board {
    pub fn gen_moves<const ALL: bool>(&self) -> MoveList {
        let mut ml = MoveList::default();

        match self.state.checkers.nbits() {
            // No checkers: enumerate all moves.
            0 => {
                self.enumerate_pawn::<ALL, false>(&mut ml);
                self.enumerate_knight::<ALL, false>(&mut ml);
                self.enumerate_diag::<ALL, false>(&mut ml);
                self.enumerate_orth::<ALL, false>(&mut ml);
                self.enumerate_king::<ALL, false>(&mut ml);
            }

            // 1 checker: enumerate all moves in the checkmask.
            1 => {
                self.enumerate_pawn::<ALL, true>(&mut ml);
                self.enumerate_knight::<ALL, true>(&mut ml);
                self.enumerate_diag::<ALL, true>(&mut ml);
                self.enumerate_orth::<ALL, true>(&mut ml);
                self.enumerate_king::<ALL, true>(&mut ml);
            }

            // 2 pieces checking king: we can't block both, we have to escape.
            2 => {
                self.enumerate_king::<ALL, true>(&mut ml);
            }

            // Cannot have more than 2 checkers in position.
            _ => unreachable!(),
        }

        ml
    }

    /// Add all moves from a given square.
    #[inline]
    fn add_moves<const ALL: bool, const CHECK: bool>(&self, from: Square, mut dest: Bitboard, ml: &mut MoveList) {
        if CHECK {
            dest &= self.state.checkmask;
        }

        (dest & self.c_bb(!self.stm)).bitloop(|to| {
            ml.push(Move::new(from, to, MoveFlag::Capture));
        });

        if ALL {
            (dest & !self.occ()).bitloop(|to| {
                ml.push(Move::new(from, to, MoveFlag::Normal));
            });
        }
    }

    /// Generate all quiet promotions.
    #[inline]
    fn add_promos<const ALL: bool>(&self, src: Square, tgt: Square, ml: &mut MoveList) {
        ml.push(Move::new(src, tgt, MoveFlag::PromoQ));

        if ALL {
            ml.push(Move::new(src, tgt, MoveFlag::PromoN));
            ml.push(Move::new(src, tgt, MoveFlag::PromoR));
            ml.push(Move::new(src, tgt, MoveFlag::PromoB));
        }
    }

    /// Generate all capture promotions.
    #[inline]
    fn add_cpromos(&self, src: Square, tgt: Square, ml: &mut MoveList) {
        ml.push(Move::new(src, tgt, MoveFlag::CPromoQ));
        ml.push(Move::new(src, tgt, MoveFlag::CPromoN));
        ml.push(Move::new(src, tgt, MoveFlag::CPromoR));
        ml.push(Move::new(src, tgt, MoveFlag::CPromoB));
    }

    /// Add all pawn moves.
    #[inline]
    fn enumerate_pawn<const ALL: bool, const CHECK: bool>(&self, ml: &mut MoveList) {
        let up = Direction::up(self.stm);
        let ul = Direction::ul(self.stm);
        let ur = Direction::ur(self.stm);

        let diag = self.state.pin_diag;
        let orth = self.state.pin_orth;

        let opps = self.c_bb(!self.stm);

        let pawns = self.pc_bb(self.stm, Piece::Pawn);

        // Promotions.
        let promo = pawns & Bitboard::PR[self.stm.index()] & !self.state.pin_orth;
        if !promo.is_empty() {
            // With capture. We can move within pinmask as long as we stay within pinmask.
            let mut cl = ((promo & !diag).shift(ul) | (promo & diag).shift(ul) & diag) & opps;
            let mut cr = ((promo & !diag).shift(ur) | (promo & diag).shift(ur) & diag) & opps;

            if CHECK {
                cl &= self.state.checkmask;
                cr &= self.state.checkmask;
            }

            cl.bitloop(|s| {
                self.add_cpromos(s.sub_dir(ul), s, ml);
            });

            cr.bitloop(|s| {
                self.add_cpromos(s.sub_dir(ur), s, ml);
            });

            // Without capture
            let mut bb = (promo & !diag).shift(up) & !self.occ();

            if CHECK {
                bb &= self.state.checkmask;
            }

            bb.bitloop(|s| {
                self.add_promos::<ALL>(s.sub_dir(up), s, ml);
            });
        }

        // En passants.
        if self.state.epsq != Square::Invalid {
            let eprank = Bitboard::EP[self.stm.index()];
            let epcap = self.state.epsq.sub_dir(up).bb();
            let mut epbb = pawn_atk(!self.stm, self.state.epsq) & pawns & !orth;

            // If we are in check, only add checkmask if it is not the ep piece putting us in
            // check.
            if CHECK && (self.state.checkers & epcap).is_empty() {
                epbb &= self.state.checkmask;
            }

            epbb.bitloop(|src| {
                // Our pawn is pinned but taking enemy pawn makes us leave pinmask
                if !(src.bb() & diag).is_empty() && (self.state.epsq.bb() & diag).is_empty() {
                    return;
                }

                // Prune orth pins
                // 1. Do a quick first check to make sure that our king and opponent's orthogonal
                //    sliders are not on ep rank
                // 2. If this first check passes, then make sure that our king is not in danger
                //    after the move happens and both pieces are removed from the rank.
                if (eprank & self.pc_bb(self.stm, Piece::King)).is_empty() && (eprank & self.orth_slider(!self.stm)).is_empty()
                    || (eprank & rook_atk(self.ksq(self.stm), self.occ() ^ src.bb() ^ epcap) & self.orth_slider(!self.stm)).is_empty()
                {
                    ml.push(Move::new(src, self.state.epsq, MoveFlag::EnPassant));
                }
            });
        }

        // Pushes.
        if ALL {
            let p = pawns & !promo & !diag;
            let mut singles = ((p & !orth).shift(up) | (p & orth).shift(up) & orth) & !self.occ();
            let mut doubles = (Bitboard::DP[self.stm.index()] & singles).shift(up) & !self.occ();

            if CHECK {
                singles &= self.state.checkmask;
                doubles &= self.state.checkmask;
            }

            singles.bitloop(|s| {
                ml.push(Move::new(s.sub_dir(up), s, MoveFlag::Normal));
            });

            doubles.bitloop(|s| {
                ml.push(Move::new(s.sub_dir(up).sub_dir(up), s, MoveFlag::DoublePush));
            });
        }

        // Captures.
        let p = pawns & !promo & !orth;
        let mut cl = ((p & !diag).shift(ul) | (p & diag).shift(ul) & diag) & opps;
        let mut cr = ((p & !diag).shift(ur) | (p & diag).shift(ur) & diag) & opps;

        if CHECK {
            cl &= self.state.checkmask;
            cr &= self.state.checkmask;
        }

        cl.bitloop(|s| {
            ml.push(Move::new(s.sub_dir(ul), s, MoveFlag::Capture));
        });

        cr.bitloop(|s| {
            ml.push(Move::new(s.sub_dir(ur), s, MoveFlag::Capture));
        });
    }

    /// Add all king moves.
    #[inline]
    fn enumerate_king<const ALL: bool, const CHECK: bool>(&self, ml: &mut MoveList) {
        let ksq = self.ksq(self.stm);

        // Generate regular king moves (assume not in check: we want to stay out of the checkmask!)
        self.add_moves::<ALL, false>(ksq, king_atk(ksq) & !self.state.attacked, ml);

        if !CHECK {
            // Generate castling moves.
            let occ = self.occ();
            let atk = self.state.attacked;
            let ks_pin = Square::G1.relative(self.stm).bb() & self.state.pin_orth;
            let qs_pin = (Square::C1.relative(self.stm).bb() | Square::B1.relative(self.stm).bb()) & self.state.pin_orth;

            // Kingside
            if self.state.castling.has_ks(self.stm) && ks_pin.is_empty() {
                let (occ_mask, atk_mask) = self.castlingmask.occ_atk::<true>(ksq, self.stm);
                if (occ & occ_mask).is_empty() && (atk & atk_mask).is_empty() {
                    ml.push(Move::new(ksq, Square::G1.relative(self.stm), MoveFlag::Castling));
                }
            }

            // Queenside
            if self.state.castling.has_qs(self.stm) && qs_pin.is_empty() {
                let (occ_mask, atk_mask) = self.castlingmask.occ_atk::<false>(ksq, self.stm);
                if (occ & occ_mask).is_empty() && (atk & atk_mask).is_empty() {
                    ml.push(Move::new(ksq, Square::C1.relative(self.stm), MoveFlag::Castling));
                }
            }
        }
    }

    /// Add all knight moves.
    #[inline]
    fn enumerate_knight<const ALL: bool, const CHECK: bool>(&self, ml: &mut MoveList) {
        // Pinned knights can never move: they move diagonally and orthogonally at once.
        let knights = self.pc_bb(self.stm, Piece::Knight) & !(self.state.pin_diag | self.state.pin_orth);

        knights.bitloop(|s| {
            self.add_moves::<ALL, CHECK>(s, knight_atk(s) & !self.c_bb(self.stm), ml);
        });
    }

    /// Add all diagonal slider moves.
    #[inline]
    fn enumerate_diag<const ALL: bool, const CHECK: bool>(&self, ml: &mut MoveList) {
        let diag = self.diag_slider(self.stm) & !self.state.pin_orth;
        let occ = self.occ();
        let ok = !self.c_bb(self.stm);

        // Non pinned bishop + queen.
        (diag & !self.state.pin_diag).bitloop(|s| {
            self.add_moves::<ALL, CHECK>(s, bishop_atk(s, occ) & ok, ml);
        });

        // Pinned bishop + queen.
        (diag & self.state.pin_diag).bitloop(|s| {
            self.add_moves::<ALL, CHECK>(s, bishop_atk(s, occ) & ok & self.state.pin_diag, ml);
        });
    }

    // Add all orthogonal slider moves.
    #[inline]
    fn enumerate_orth<const ALL: bool, const CHECK: bool>(&self, ml: &mut MoveList) {
        let orth = self.orth_slider(self.stm) & !self.state.pin_diag;
        let occ = self.occ();
        let ok = !self.c_bb(self.stm);

        // Non pinned rook + queen.
        (orth & !self.state.pin_orth).bitloop(|s| {
            self.add_moves::<ALL, CHECK>(s, rook_atk(s, occ) & ok, ml);
        });

        // Pinned rook + queen.
        (orth & self.state.pin_orth).bitloop(|s| {
            self.add_moves::<ALL, CHECK>(s, rook_atk(s, occ) & ok & self.state.pin_orth, ml);
        });
    }
}
