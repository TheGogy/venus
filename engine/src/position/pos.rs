use chess::types::{board::Board, moves::Move};

use crate::threading::thread::Thread;

/// Position.
/// This provides a wrapper around the Board struct for full engine use, to clean up
/// things like making a move picker, etc.
#[derive(Clone, Debug)]
pub struct Pos {
    pub board: Board,
}

impl Default for Pos {
    fn default() -> Self {
        "startpos".parse().unwrap()
    }
}

/// Get a position from a string.
impl std::str::FromStr for Pos {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split_whitespace();
        let mut board: Board = match tokens.next() {
            Some("startpos") => Board::default(),
            Some("kiwipete") => "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1".parse().unwrap(),
            Some("fen") => {
                let fen = &tokens.clone().take(6).collect::<Vec<&str>>().join(" ")[..];

                for _ in 0..6 {
                    tokens.next();
                }

                fen.parse()?
            }
            _ => return Err("Invalid position!"),
        };

        if let Some("moves") = tokens.next() {
            for move_str in tokens {
                let m = board.find_move(move_str);

                match m {
                    Some(m) => board.make_move(m),
                    None => return Err("Invalid move!"),
                };
            }
        };

        Ok(Self { board })
    }
}

impl Pos {
    pub fn make_move(&mut self, m: Move, t: &mut Thread) {
        let p = self.board.pc_at(m.src());
        t.move_made(p, m);
        self.board.make_move(m);
    }

    pub fn make_null(&mut self, t: &mut Thread) {
        t.null_made();
        self.board.make_null();
    }

    pub fn undo_move(&mut self, m: Move, t: &mut Thread) {
        t.move_undo();
        self.board.undo_move(m);
    }
}
