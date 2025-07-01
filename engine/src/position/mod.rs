pub mod eval;

use chess::types::{board::Board, moves::Move};
use nnue::network::NNUE;

use crate::{history::conthist::PieceTo, threading::thread::Thread};

/// Position.
/// This contains a representation of the board itself and the NNUE updated with the most recently
/// evaluated board.
#[derive(Clone, Debug)]
pub struct Position {
    pub board: Board,
    nnue: NNUE,
}

impl Default for Position {
    fn default() -> Self {
        "startpos".parse().unwrap()
    }
}

/// Get a position from a string.
impl std::str::FromStr for Position {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split_whitespace();

        let mut board: Board = match tokens.next() {
            Some("startpos") => Board::default(),

            // Testing positions
            Some("kiwipete") => "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1".parse().unwrap(),
            Some("killer") => "rnbqkb1r/pp1p1pPp/8/2p1pP2/1P1P4/3P3P/P1P1P3/RNBQKBNR w KQkq e6 0 1".parse().unwrap(),
            Some("nolot3") => "r2qk2r/ppp1b1pp/2n1p3/3pP1n1/3P2b1/2PB1NN1/PP4PP/R1BQK2R w KQkq - 0 1".parse().unwrap(),
            Some("nolot9") => "r4r1k/4bppb/2n1p2p/p1n1P3/1p1p1BNP/3P1NP1/qP2QPB1/2RR2K1 w - - 0 1".parse().unwrap(),
            Some("tricky") => "3qk1b1/1p4r1/1n4r1/2P1b2B/p3N2p/P2Q3P/8/1R3R1K w - - 2 39".parse().unwrap(),
            Some("endgame") => "r7/6k1/1p6/2pp1p2/7Q/8/p1P2K1P/8 w - - 0 32".parse().unwrap(),

            // FEN parsing
            Some("fen") => {
                let fen = &tokens.clone().take(6).collect::<Vec<&str>>().join(" ")[..];

                for _ in 0..6 {
                    tokens.next();
                }

                fen.parse()?
            }
            _ => return Err("Invalid position!"),
        };

        // Move parsing
        if let Some("moves") = tokens.next() {
            for move_str in tokens {
                let m = board.find_move(move_str);

                match m {
                    Some(m) => board.make_move(m),
                    None => return Err("Invalid move!"),
                };
            }
        };

        Ok(Self { board, nnue: NNUE::default() })
    }
}

impl Position {
    /// Make a null move on the board on the given thread.
    pub fn make_move(&mut self, m: Move, t: &mut Thread) {
        t.move_made(PieceTo::from(&self.board, m));
        self.board.make_move(m);
    }

    /// Undo a move on the board on a given thread.
    pub fn undo_move(&mut self, m: Move, t: &mut Thread) {
        t.move_undo();
        self.board.undo_move(m);
    }

    /// Make a null move on the board on a given thread.
    pub fn make_null(&mut self, t: &mut Thread) {
        t.null_made();
        self.board.make_null();
    }

    /// Make a null move on the board on a given thread.
    pub fn undo_null(&mut self, t: &mut Thread) {
        t.move_undo();
        self.board.undo_null();
    }
}
