use crate::types::{board::Board, piece::Piece};

/// Draw implementations for board.
impl Board {
    /// Whether the current position leads to a draw.
    pub fn is_draw(&self, ply_from_null: usize) -> bool {
        self.is_fifty_move() || self.is_insufficient_material() || self.is_repetition(ply_from_null)
    }

    /// Whether the 50 move rule has been passed.
    #[inline]
    fn is_fifty_move(&self) -> bool {
        self.state.halfmoves >= 100
    }

    /// Whether the current position has insufficient material to win for either side.
    #[inline]
    fn is_insufficient_material(&self) -> bool {
        match self.occ().nbits() {
            // King vs King => draw.
            2 => true,

            // King vs King + (...) => Draw if other piece is a knight or bishop.
            3 => !(self.p_bb(Piece::Knight) | self.p_bb(Piece::Bishop)).is_empty(),

            // Otherwise, assume winnable
            _ => false,
        }
    }

    /// Whether the current position has been repeated.
    #[inline]
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
