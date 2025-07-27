use crate::types::{bitboard::Bitboard, board::Board, color::Color, piece::Piece};

/// Draw implementations for board.
impl Board {
    /// Whether the current position leads to a draw.
    pub fn is_draw(&self, ply_from_null: usize) -> bool {
        self.is_fifty_move() || self.is_insufficient_material() || self.is_repetition(ply_from_null)
    }

    /// Whether the 50 move rule has been passed.
    const fn is_fifty_move(&self) -> bool {
        self.state.halfmoves >= 100
    }

    /// Whether the current position has insufficient material to win for either side.
    fn is_insufficient_material(&self) -> bool {
        let n_pcs = self.occ().nbits();

        match n_pcs {
            // King vs King => draw.
            2 => true,

            // King vs King + (...) => Draw if other piece is a Knight or Bishop.
            3 => (self.p_bb(Piece::Knight) | self.p_bb(Piece::Bishop)).any(),

            // Anything else.
            _ => {
                // If we have a piece that can checkmate, it is not a draw.
                if (self.p_bb(Piece::Pawn) | self.p_bb(Piece::Rook) | self.p_bb(Piece::Queen)).any() {
                    return false;
                }

                // The number of non-King pieces on the board (number of Knights and Bishops).
                // We know that exactly 2 Kings must exist, and no other pieces exist.
                let n_minor_pcs = n_pcs - 2;

                // 2 minor pieces.
                // We can checkmate with Bishop + Knight, or we can checkmate with 2 Bishops of
                // opposite types.
                if n_minor_pcs == 2 {
                    // If we have one minor piece each, then we cannot force checkmate.
                    if self.c_bb(Color::White).nbits() == self.c_bb(Color::Black).nbits() {
                        return true;
                    }

                    let knights = self.p_bb(Piece::Knight);
                    let bishops = self.p_bb(Piece::Bishop);

                    // 2 Knights can technically deliver checkmate, though this cannot be forced.
                    // https://lichess.org/editor/8/8/8/8/8/1N2N3/8/3k1K2_b_-_-_0_3?color=white
                    if knights.any() && !bishops.any() {
                        return knights.nbits() <= 2;
                    }

                    // 2 Bishops can deliver checkmate (except in the rare case that we have
                    // underpromoted to a bishop and now have 2 bishops of the same color)
                    if bishops.any() && !knights.any() {
                        return bishops & Bitboard::WHITE_SQ == bishops || bishops & Bitboard::BLACK_SQ == bishops;
                    }
                }

                // We have more than 2 minor pieces: this can lead to checkmate.
                false
            }
        }
    }

    /// Whether the current position has been repeated.
    fn is_repetition(&self, ply_from_null: usize) -> bool {
        let end = 1 + ply_from_null.min(self.state.halfmoves);

        if end == 1 {
            return false;
        }

        let key = self.state.hash.key;
        self.history.iter().rev().take(end).skip(1).step_by(2).any(|s| s.hash.key == key)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        board::Board,
        moves::{Move, MoveFlag},
        square::Square,
    };

    #[test]
    fn test_fifty_move() {
        let b: Board = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 100 1".parse().unwrap();
        assert!(b.is_fifty_move());

        let b: Board = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".parse().unwrap();
        assert!(!b.is_fifty_move());
    }

    #[test]
    fn test_insufficient_material() {
        let b: Board = "8/8/8/5k2/2K5/8/8/8 w - - 0 1".parse().unwrap();
        assert!(b.is_insufficient_material());

        let b: Board = "8/8/8/2N2k2/2K5/8/8/8 w - - 0 1".parse().unwrap();
        assert!(b.is_insufficient_material());

        let b: Board = "8/8/8/2P2k2/2K5/8/8/8 w - - 0 1".parse().unwrap();
        assert!(!b.is_insufficient_material());
    }

    #[test]
    fn test_repetition() {
        let mut b = Board::default();
        let h = b.state.hash.key;

        b.make_move(Move::new(Square::G1, Square::F3, MoveFlag::Normal));
        b.make_move(Move::new(Square::G8, Square::F6, MoveFlag::Normal));
        b.make_move(Move::new(Square::F3, Square::G1, MoveFlag::Normal));
        b.make_move(Move::new(Square::F6, Square::G8, MoveFlag::Normal));

        assert_eq!(b.state.hash.key, h);

        b.make_move(Move::new(Square::G1, Square::F3, MoveFlag::Normal));
        b.make_move(Move::new(Square::G8, Square::F6, MoveFlag::Normal));
        b.make_move(Move::new(Square::F3, Square::G1, MoveFlag::Normal));
        b.make_move(Move::new(Square::F6, Square::G8, MoveFlag::Normal));

        assert_eq!(b.state.hash.key, h);

        b.make_move(Move::new(Square::G1, Square::F3, MoveFlag::Normal));
        b.make_move(Move::new(Square::G8, Square::F6, MoveFlag::Normal));
        b.make_move(Move::new(Square::F3, Square::G1, MoveFlag::Normal));
        b.make_move(Move::new(Square::F6, Square::G8, MoveFlag::Normal));

        assert_eq!(b.state.hash.key, h);

        assert!(b.is_repetition(9));
    }
}
