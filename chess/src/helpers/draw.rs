use crate::types::{board::Board, color::Color, piece::Piece};

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
    /// A position is drawn unless:
    fn is_insufficient_material(&self) -> bool {
        // 1. A piece that can deliver checkmate exists on the board.
        let winning_piece = (self.p_bb(Piece::Pawn) | self.p_bb(Piece::Rook) | self.p_bb(Piece::Queen)).any();

        // 2. Both sides have multiple pieces.
        let multiple_pieces = self.c_bb(Color::White).multiple() && self.c_bb(Color::Black).multiple();

        // 3a. Multiple minor pieces exist on the board (EXCEPT for two knights, handled below).
        let multiple_minor = (self.p_bb(Piece::Knight) | self.p_bb(Piece::Bishop)).multiple();

        // 3b. If we don't have any bishops, we need at least 3 knights to force checkmate.
        //     (2 knights can checkmate on the edge of the board, but search will prevent it)
        //     https://lichess.org/editor/8/8/8/8/8/1n2n3/8/3K1k2_w_-_-_0_1?color=white
        let too_few_knights = !self.p_bb(Piece::Bishop).any() && self.p_bb(Piece::Knight).nbits() < 3;

        !winning_piece && !multiple_pieces && (!multiple_minor || too_few_knights)
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
