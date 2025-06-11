use crate::{
    tables::{
        leaping_piece::{all_pawn_atk, king_atk, knight_atk, pawn_atk},
        sliding_piece::{between, bishop_atk, rook_atk},
    },
    types::{
        bitboard::Bitboard,
        board::{Board, BoardState},
        piece::Piece,
    },
};

/// Contains functions for updating the masks within the provided board state.
impl Board {
    /// Updates the masks for the given board state.
    pub(crate) fn update_masks(&self, state: &mut BoardState) {
        self.update_attacked(state);
        self.update_checkers(state);
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
        self.diag_slider(opp).bitloop(|s| {
            state.attacked |= bishop_atk(s, occ);
        });

        // Rooks + Queens.
        self.orth_slider(opp).bitloop(|s| {
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
          | self.diag_slider(opp)          & bishop_atk(ksq, occ)
          | self.orth_slider(opp)          & rook_atk(ksq, occ)
    }

    /// Update the pins on the board.
    #[rustfmt::skip]
    fn update_pins(&self, state: &mut BoardState) {
        let opp = !self.stm;
        let ksq = self.ksq(self.stm);
        let stm_occ = self.c_bb(self.stm);
        let opp_occ = self.c_bb(opp);

        state.checkmask = Bitboard::EMPTY;
        state.pin_diag = Bitboard::EMPTY;
        state.pin_orth = Bitboard::EMPTY;

        // We have already determined if we are in check with update_checkers; don't do these
        // lookups unless absolutely necessary.
        if state.checkers.any() {
            state.checkmask = self.pc_bb(opp, Piece::Pawn) & pawn_atk(self.stm, ksq)
                            | self.pc_bb(opp, Piece::Knight) & knight_atk(ksq)
        }

        // Bishops and queens
        (self.diag_slider(opp) & bishop_atk(ksq, opp_occ)).bitloop(|s| {
            let between = between(ksq, s);
            match (between & stm_occ).nbits() {
                0 => state.checkmask |= between | s.bb(), // No pieces: add to checkmask
                1 => state.pin_diag  |= between | s.bb(), // One piece: add to pinmask
                _ => {}                                   // > 1 piece: do nothing
            }
        });

        // Rooks and queens
        (self.orth_slider(opp) & rook_atk(ksq, opp_occ)).bitloop(|s| {
            let between = between(ksq, s);
            match (between & stm_occ).nbits() {
                0 => state.checkmask |= between | s.bb(), // No pieces: add to checkmask
                1 => state.pin_orth  |= between | s.bb(), // One piece: add to pinmask
                _ => {}                                   // > 1 piece: do nothing
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_bitboard_eq,
        types::{bitboard::Bitboard, board::Board},
    };

    #[test]
    fn test_update_masks_checkers() {
        let board: Board = "rnbqk1nr/pppp1Bpp/8/2b1p3/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 0 1".parse().unwrap();

        assert_bitboard_eq!(board.state.checkers, Bitboard(9007199254740992));
    }

    #[test]
    fn test_update_masks_pin_diag() {
        let board: Board = "rnbqk1nr/pppp1ppp/8/1B2p3/1b2P3/8/PPPP1PPP/RNBQK1NR w KQkq - 0 1".parse().unwrap();
        assert_bitboard_eq!(board.state.pin_diag, Bitboard(33818624));
        assert_bitboard_eq!(board.state.pin_orth, Bitboard(0));
    }

    #[test]
    fn test_update_masks_pin_orth() {
        let board: Board = "rnb1kbnr/ppppqppp/8/8/3pP3/8/PPP2PPP/RNBQKBNR w KQkq - 0 1".parse().unwrap();

        assert_bitboard_eq!(board.state.pin_diag, Bitboard(0));
        assert_bitboard_eq!(board.state.pin_orth, Bitboard(4521260802379776));
    }

    #[test]
    fn test_update_masks_checkmask() {
        let board: Board = "rnbqk1nr/pppp2pp/3N2B1/2b1p3/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 0 1".parse().unwrap();

        assert_bitboard_eq!(board.state.checkers, Bitboard(79164837199872));
        assert_bitboard_eq!(board.state.checkmask, Bitboard(9086364091940864));
    }
}
