use crate::{
    tables::{
        leaping_piece::{all_pawn_atk, king_atk, knight_atk, pawn_atk},
        sliding_piece::{between, bishop_atk, rook_atk},
    },
    types::{
        bitboard::Bitboard,
        board::{Board, BoardState},
        color::Color,
        piece::Piece,
    },
};

/// Contains functions for updating the masks within the provided board state.
impl Board {
    /// Updates the masks for the given board state.
    pub(crate) fn update_masks(&self, state: &mut BoardState) {
        self.update_attacked(state);
        self.update_checkers(state);
        self.update_kinglines(state);
        self.update_pins(state);
    }

    /// Updates the attacked pieces mask.
    fn update_attacked(&self, state: &mut BoardState) {
        let opp = !self.stm;
        let occ = self.occ() ^ self.pc_bb(self.stm, Piece::King);

        // Pawns.
        state.attacked = all_pawn_atk(self.pc_bb(opp, Piece::Pawn), opp);

        // Knights.
        self.pc_bb(opp, Piece::Knight).bitloop(|s| {
            state.attacked |= knight_atk(s);
        });

        // Bishops + Queens.
        self.diag_bb(opp).bitloop(|s| {
            state.attacked |= bishop_atk(s, occ);
        });

        // Rooks + Queens.
        self.orth_bb(opp).bitloop(|s| {
            state.attacked |= rook_atk(s, occ);
        });

        // King.
        state.attacked |= king_atk(self.ksq(opp));
    }

    /// Update the king lines and checkers.
    #[rustfmt::skip]
    fn update_checkers(&self, state: &mut BoardState) {
        let opp = !self.stm;
        let ksq = self.ksq(self.stm);
        let occ = self.occ();

        state.checkers =
            self.pc_bb(opp, Piece::Pawn)   & pawn_atk(self.stm, ksq)
          | self.pc_bb(opp, Piece::Knight) & knight_atk(ksq)
          | self.diag_bb(opp)               & bishop_atk(ksq, occ)
          | self.orth_bb(opp)               & rook_atk(ksq, occ)
    }

    /// Update the king lines.
    #[rustfmt::skip]
    fn update_kinglines(&self, state: &mut BoardState) {
        let ksq = self.ksq(!self.stm);
        let occ = self.occ();

        state.kinglines[0] = pawn_atk(!self.stm, ksq);                // Pawn
        state.kinglines[1] = knight_atk(ksq);                         // Knight
        state.kinglines[2] = bishop_atk(ksq, occ);                    // Bishop
        state.kinglines[3] = rook_atk(ksq, occ);                      // Rook
        state.kinglines[4] = state.kinglines[2] | state.kinglines[3]; // Queen
    }

    /// Update the pins on the board.
    #[rustfmt::skip]
    fn update_pins(&self, state: &mut BoardState) {
        let opp = !self.stm;
        let stm_occ = self.c_bb(self.stm);
        let opp_occ = self.c_bb(opp);

        let ksqs = [self.ksq(Color::White), self.ksq(Color::Black)];

        state.checkmask = Bitboard::EMPTY;
        state.pin_diag = [Bitboard::EMPTY; Color::NUM];
        state.pin_orth = [Bitboard::EMPTY; Color::NUM];

        // We have already determined if we are in check with update_checkers; don't do these
        // lookups unless absolutely necessary.
        if state.checkers.any() {
            state.checkmask = self.pc_bb(opp, Piece::Pawn) & pawn_atk(self.stm, ksqs[self.stm.idx()])
                            | self.pc_bb(opp, Piece::Knight) & knight_atk(ksqs[self.stm.idx()])
        }

        // Bishops and queens
        (self.diag_bb(opp) & bishop_atk(ksqs[self.stm.idx()], opp_occ)).bitloop(|s| {
            let between = between(ksqs[self.stm.idx()], s);
            match (between & stm_occ).nbits() {
                0 => state.checkmask                |= between | s.bb(), // No pieces: add to checkmask
                1 => state.pin_diag[self.stm.idx()] |= between | s.bb(), // One piece: add to pinmask
                _ => {}                                                  // > 1 piece: do nothing
            }
        });

        // Rooks and queens
        (self.orth_bb(opp) & rook_atk(ksqs[self.stm.idx()], opp_occ)).bitloop(|s| {
            let between = between(ksqs[self.stm.idx()], s);
            match (between & stm_occ).nbits() {
                0 => state.checkmask                |= between | s.bb(), // No pieces: add to checkmask
                1 => state.pin_orth[self.stm.idx()] |= between | s.bb(), // One piece: add to pinmask
                _ => {}                                                  // > 1 piece: do nothing
            }
        });

        // Update pinmasks for opponent.
        (self.diag_bb(opp) & bishop_atk(ksqs[opp.idx()], opp_occ)).bitloop(|s| {
            let between = between(ksqs[opp.idx()], s);
            if (between & stm_occ).nbits() == 1 { state.pin_diag[opp.idx()] |= between | s.bb() }
        });

        (self.orth_bb(opp) & rook_atk(ksqs[opp.idx()], opp_occ)).bitloop(|s| {
            let between = between(ksqs[opp.idx()], s);
            if (between & stm_occ).nbits() == 1 { state.pin_orth[opp.idx()] |= between | s.bb() }
        });
    }
}
