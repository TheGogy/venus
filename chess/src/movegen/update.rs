use crate::{
    tables::{
        leaping_piece::{knight_atk, pawn_atk},
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
    pub fn update_masks(&self, state: &mut BoardState) {
        self.update_attacked(state);
        self.update_checkers(state);
        self.update_kinglines(state);
        self.update_pins(state);
    }

    /// Updates the attacked pieces mask.
    fn update_attacked(&self, state: &mut BoardState) {
        let opp = !self.stm;
        let occ = self.occ() ^ self.pc_bb(self.stm, Piece::King);

        // Pawns, knights, king.
        state.attacked = self.all_pawn_atk(opp) | self.all_knight_atk(opp) | self.all_king_atk(opp);

        // Bishops + Queens.
        for s in self.diag_bb(opp) {
            state.attacked |= bishop_atk(s, occ);
        }

        // Rooks + Queens.
        for s in self.orth_bb(opp) {
            state.attacked |= rook_atk(s, occ);
        }
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
          | self.diag_bb(opp)              & bishop_atk(ksq, occ)
          | self.orth_bb(opp)              & rook_atk(ksq, occ)
    }

    /// Update the king lines.
    fn update_kinglines(&self, state: &mut BoardState) {
        let ksq = self.ksq(!self.stm);
        let occ = self.occ();

        state.kinglines[0] = pawn_atk(!self.stm, ksq);
        state.kinglines[1] = knight_atk(ksq);
        state.kinglines[2] = bishop_atk(ksq, occ);
        state.kinglines[3] = rook_atk(ksq, occ);
        state.kinglines[4] = state.kinglines[2] | state.kinglines[3];
    }

    /// Update the pins on the board.
    fn update_pins(&self, state: &mut BoardState) {
        let opp = !self.stm;
        let stm_occ = self.c_bb(self.stm);
        let opp_occ = self.c_bb(opp);

        let ksqs = [self.ksq(Color::White), self.ksq(Color::Black)];

        state.pin_diag = [Bitboard::EMPTY; Color::NUM];
        state.pin_orth = [Bitboard::EMPTY; Color::NUM];

        // Pawns and knights.
        state.checkmask = self.pc_bb(opp, Piece::Pawn) & pawn_atk(self.stm, ksqs[self.stm.idx()])
            | self.pc_bb(opp, Piece::Knight) & knight_atk(ksqs[self.stm.idx()]);

        // Bishops and queens
        for s in self.diag_bb(opp) & bishop_atk(ksqs[self.stm.idx()], opp_occ) {
            let between = between(ksqs[self.stm.idx()], s);
            match (between & stm_occ).nbits() {
                0 => state.checkmask |= between | s.bb(),                // No pieces: add to checkmask.
                1 => state.pin_diag[self.stm.idx()] |= between | s.bb(), // One piece: add to pinmask.
                _ => {}                                                  // > 1 piece: do nothing.
            }
        }

        // Rooks and queens.
        for s in self.orth_bb(opp) & rook_atk(ksqs[self.stm.idx()], opp_occ) {
            let between = between(ksqs[self.stm.idx()], s);
            match (between & stm_occ).nbits() {
                0 => state.checkmask |= between | s.bb(),                // No pieces: add to checkmask.
                1 => state.pin_orth[self.stm.idx()] |= between | s.bb(), // One piece: add to pinmask.
                _ => {}                                                  // > 1 piece: do nothing.
            }
        }

        // Update pinmasks for opponent.
        for s in self.diag_bb(opp) & bishop_atk(ksqs[opp.idx()], stm_occ) {
            let between = between(ksqs[opp.idx()], s);
            if (between & opp_occ).nbits() == 1 {
                state.pin_diag[opp.idx()] |= between | s.bb()
            }
        }

        for s in self.orth_bb(opp) & rook_atk(ksqs[opp.idx()], stm_occ) {
            let between = between(ksqs[opp.idx()], s);
            if (between & opp_occ).nbits() == 1 {
                state.pin_orth[opp.idx()] |= between | s.bb()
            }
        }
    }
}
