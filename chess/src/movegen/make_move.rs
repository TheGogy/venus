use crate::types::{
    board::Board,
    moves::{Move, MoveFlag},
    piece::{CPiece, Piece},
    square::Square,
};

/// Make and unmake move functions.
/// WARN: Assumes the move is legal in this position.
impl Board {
    /// Make a move in the current position.
    pub fn make_move(&mut self, m: Move) {
        let flag = m.flag();
        let (src, tgt) = (m.src(), m.tgt());
        let mut piece = self.get_piece(src);

        // Clone current state
        let mut state = self.state.clone();

        // Increment fullmove counter
        state.fullmoves += self.stm.index();

        // Unset ep sq
        state.epsq = Square::Invalid;

        // Remove piece from source square.
        self.pop_piece(src);

        // Do parts of move that do not include moving the piece.
        match flag {
            // Normal move: increment halfmove clock.
            MoveFlag::Normal => {
                if piece.pt() == Piece::Pawn {
                    state.halfmoves = 0
                } else {
                    state.halfmoves += 1
                }
            }

            // Castle: move rook to castling square.
            MoveFlag::Castling => {
                let (rf, rt) = self.castlingmask.rook_from_to(tgt);
                self.pop_piece(rf);
                self.set_piece(CPiece::create(self.stm, Piece::Rook), rt);
                state.halfmoves += 1;
            }

            // Double push: update epsq.
            MoveFlag::DoublePush => {
                let epsq = src.forward(self.stm);
                state.epsq = epsq;
                state.key.toggle_ep(epsq);
                state.halfmoves += 1;
            }

            // Capture: Remove piece at target square.
            MoveFlag::Capture => {
                state.cap = self.get_piece(tgt);
                self.pop_piece(tgt);
                state.halfmoves = 0;
            }

            // En passant: Remove ep captured piece.
            MoveFlag::EnPassant => {
                let epsq = tgt.forward(!self.stm);
                state.cap = self.get_piece(epsq);
                self.pop_piece(epsq);
                state.halfmoves = 0;
            }

            // Regular promotion: set piece to promoted piece.
            MoveFlag::PromoN | MoveFlag::PromoB | MoveFlag::PromoR | MoveFlag::PromoQ => {
                piece = CPiece::create(self.stm, flag.get_promo());
                state.halfmoves = 0;
            }

            // Capture promotion: remove piece from to square and set piece to promoted piece.
            MoveFlag::CPromoN | MoveFlag::CPromoB | MoveFlag::CPromoR | MoveFlag::CPromoQ => {
                state.cap = self.get_piece(tgt);
                self.pop_piece(tgt);
                piece = CPiece::create(self.stm, flag.get_promo());
                state.halfmoves = 0;
            }
        }

        // Zero out bits in castling mask
        state.key.toggle_castling(state.castling);
        state.castling &= self.castlingmask.zero_out(src, tgt);
        state.key.toggle_castling(state.castling);

        // Set piece on target square.
        self.set_piece(piece, tgt);

        // Update stm.
        self.stm = !self.stm;
        state.key.toggle_color();

        // Update masks for movegen.
        self.update_masks(&mut state);

        // Set current state and push old state to history.
        let old_state = std::mem::replace(&mut self.state, state);
        self.history.push(old_state);
    }

    /// Undo a move on the board.
    pub fn undo_move(&mut self, m: Move) {
        let flag = m.flag();
        let (src, tgt) = (m.src(), m.tgt());
        let mut piece = self.get_piece(tgt);
        let cap = self.state.cap;

        // SAFETY: This will only be called when there is a valid move in the history.
        let old_state = unsafe { self.history.pop().unwrap_unchecked() };
        self.state = old_state;

        // Update stm.
        self.stm = !self.stm;

        // Remove moved piece.
        self.pop_piece(tgt);

        // Do parts of move that do not include moving the piece.
        match flag {
            // Normal move and double push: do nothing.
            MoveFlag::Normal | MoveFlag::DoublePush => {}

            // Castling: move rook back.
            MoveFlag::Castling => {
                let (rf, rt) = self.castlingmask.rook_from_to(tgt);
                self.pop_piece(rt);
                self.set_piece(CPiece::create(self.stm, Piece::Rook), rf);
            }

            // Capture: replace the captured piece.
            MoveFlag::Capture => {
                self.set_piece(cap, tgt);
            }

            // EnPassant: replace the captured piece.
            MoveFlag::EnPassant => {
                let eptgt = self.state.epsq.forward(!self.stm);
                self.set_piece(cap, eptgt);
            }

            // Regular promotion: set piece to promoted piece.
            MoveFlag::PromoN | MoveFlag::PromoB | MoveFlag::PromoR | MoveFlag::PromoQ => {
                piece = CPiece::create(self.stm, Piece::Pawn);
            }

            // Capture promotion: add piece back to tgt square and set piece to promoted piece.
            MoveFlag::CPromoN | MoveFlag::CPromoB | MoveFlag::CPromoR | MoveFlag::CPromoQ => {
                self.set_piece(cap, tgt);
                piece = CPiece::create(self.stm, Piece::Pawn);
            }
        }

        // Add piece back in.
        self.set_piece(piece, src);
    }

    /// Make a null move on the board.
    pub fn make_null(&mut self) {
        let mut state = self.state.clone();

        // Update epsq
        if state.epsq != Square::Invalid {
            state.epsq = Square::Invalid;
            state.key.toggle_ep(state.epsq);
        }

        // Update stm
        self.stm = !self.stm;
        state.key.toggle_color();

        let old_state = std::mem::replace(&mut self.state, state);
        self.history.push(old_state);

        // TODO: Update masks for movegen
    }

    /// Undo a null move from the board.
    pub fn undo_null(&mut self) {
        // SAFETY: This will only be called when there is a valid move in the history.
        let old_state = unsafe { self.history.pop().unwrap_unchecked() };
        self.state = old_state;

        // Update stm
        self.stm = !self.stm;
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        board::Board,
        moves::{Move, MoveFlag},
        piece::CPiece,
        square::Square,
    };

    #[test]
    fn test_move_normal() {
        let mut b = Board::default();
        let m = Move::new(Square::E2, Square::E4, MoveFlag::Normal);

        b.make_move(m);

        assert_eq!(b.get_piece(Square::E2), CPiece::None);
        assert_eq!(b.get_piece(Square::E4), CPiece::WPawn);
        assert_eq!(b.history.len(), 1);

        b.undo_move(m);

        assert_eq!(b.get_piece(Square::E4), CPiece::None);
        assert_eq!(b.get_piece(Square::E2), CPiece::WPawn);
        assert_eq!(b.history.len(), 0);
    }

    #[test]
    fn test_move_double() {
        let mut b: Board = "rnbqkbnr/ppppppp1/8/8/7p/P7/RPPPPPPP/1NBQKBNR w Kkq - 0 3".parse().unwrap();
        let m = Move::new(Square::G2, Square::G4, MoveFlag::DoublePush);

        b.make_move(m);

        assert_eq!(b.state.epsq, Square::G3);
    }

    #[test]
    fn test_move_castle() {
        let mut b: Board = "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4".parse().unwrap();
        let m = Move::new(Square::E1, Square::G1, MoveFlag::Castling);

        b.make_move(m);

        assert_eq!(b.get_piece(Square::E1), CPiece::None);
        assert_eq!(b.get_piece(Square::G1), CPiece::WKing);

        b.undo_move(m);

        assert_eq!(b.get_piece(Square::F1), CPiece::None);
        assert_eq!(b.get_piece(Square::H1), CPiece::WRook);
    }

    #[test]
    fn test_move_cap() {
        let mut b: Board = "rnbqkbnr/ppp2ppp/4p3/3p4/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 1".parse().unwrap();
        let m = Move::new(Square::C4, Square::D5, MoveFlag::Capture);

        b.make_move(m);

        assert_eq!(b.get_piece(Square::D5), CPiece::WPawn);
        assert_eq!(b.get_piece(Square::C4), CPiece::None);

        b.undo_move(m);

        assert_eq!(b.get_piece(Square::C4), CPiece::WPawn);
        assert_eq!(b.get_piece(Square::D5), CPiece::BPawn);
    }

    #[test]
    fn test_move_ep() {
        let mut b: Board = "rnbqkbnr/ppppp1p1/7p/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3".parse().unwrap();
        let m = Move::new(Square::E5, Square::F6, MoveFlag::EnPassant);

        b.make_move(m);

        assert_eq!(b.get_piece(Square::F5), CPiece::None);
        assert_eq!(b.get_piece(Square::F6), CPiece::WPawn);
        assert_eq!(b.state.epsq, Square::Invalid);

        b.undo_move(m);

        assert_eq!(b.get_piece(Square::F5), CPiece::BPawn);
        assert_eq!(b.get_piece(Square::F6), CPiece::None);
    }

    #[test]
    fn test_move_promo() {
        let mut b: Board = "3k4/6P1/8/8/8/8/8/3K4 w - - 0 1".parse().unwrap();
        let m = Move::new(Square::G7, Square::G8, MoveFlag::PromoQ);

        b.make_move(m);

        assert_eq!(b.get_piece(Square::G8), CPiece::WQueen);
        assert_eq!(b.get_piece(Square::G7), CPiece::None);

        b.undo_move(m);

        assert_eq!(b.get_piece(Square::G7), CPiece::WPawn);
        assert_eq!(b.get_piece(Square::G8), CPiece::None);
    }

    #[test]
    fn test_move_cpromo() {
        let mut b: Board = "3k1b2/6P1/8/8/8/8/8/3K4 w - - 0 1".parse().unwrap();
        let m = Move::new(Square::G7, Square::F8, MoveFlag::CPromoQ);

        b.make_move(m);

        assert_eq!(b.get_piece(Square::F8), CPiece::WQueen);
        assert_eq!(b.get_piece(Square::G7), CPiece::None);

        b.undo_move(m);

        assert_eq!(b.get_piece(Square::G7), CPiece::WPawn);
        assert_eq!(b.get_piece(Square::F8), CPiece::BBishop);
    }

    #[test]
    fn test_pos_same() {
        let mut b = Board::default();
        let m1 = Move::new(Square::E2, Square::E4, MoveFlag::Normal);
        let m2 = Move::new(Square::E7, Square::E5, MoveFlag::Normal);
        let m3 = Move::new(Square::G1, Square::F3, MoveFlag::Normal);
        let m4 = Move::new(Square::D7, Square::D5, MoveFlag::Normal);
        let m5 = Move::new(Square::E4, Square::D5, MoveFlag::Capture);

        b.make_move(m1);
        b.make_move(m2);
        b.make_move(m3);
        b.make_move(m4);
        b.make_move(m5);

        assert_eq!(b.to_fen(), "rnbqkbnr/ppp2ppp/8/3Pp3/8/5N2/PPPP1PPP/RNBQKB1R b KQkq - 0 3");

        b.undo_move(m5);
        b.undo_move(m4);
        b.undo_move(m3);
        b.undo_move(m2);
        b.undo_move(m1);

        assert_eq!(b.to_fen(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }
}
