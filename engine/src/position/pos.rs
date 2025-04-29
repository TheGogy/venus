use chess::types::board::Board;

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
            Some("fen") => {
                let fen = &tokens.clone().take(6).collect::<Vec<&str>>().join(" ")[..];

                for _ in 0..6 {
                    tokens.next();
                }

                fen.parse()?
            }
            _ => return Err("Invalid position"),
        };

        if let Some("moves") = tokens.next() {
            for move_str in tokens {
                let m = board.find_move(move_str);

                match m {
                    Some(m) => board.make_move(m),
                    None => eprintln!("Move is not legal!"),
                };
            }
        };

        Ok(Self { board })
    }
}
