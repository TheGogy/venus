use std::str::FromStr;

use super::{
    bitboard::Bitboard,
    castling::{CastlingMask, CastlingRights},
    color::Color,
    moves::Move,
    piece::{CPiece, Piece},
    square::Square,
    zobrist::Hash,
};

/// Board State struct.
/// Contains information about the current board used to generate, make and unmake moves.
#[derive(Default, Debug, Clone)]
pub struct BoardState {
    // Current board info
    pub castling: CastlingRights,
    pub epsq: Square,
    pub halfmoves: usize,
    pub fullmoves: usize,

    // Used to unmake moves
    pub mov: Move,
    pub cap: CPiece,

    // Bitboards for movegen
    pub attacked: Bitboard,
    pub checkers: Bitboard,
    pub pindiag: Bitboard,
    pub pinorth: Bitboard,
    pub checkmask: Bitboard,

    // Keys
    pub key: Hash,
}

/// Contains the current board state.
pub struct Board {
    // Piece placement
    pub pieces: [Bitboard; Piece::NUM],
    pub colors: [Bitboard; Color::NUM],
    pub pc_map: [CPiece; Square::NUM],

    // Game state
    pub stm: Color,
    pub castlingmask: CastlingMask,

    // Board state.
    pub state: BoardState,
    pub history: Vec<BoardState>,
}

/// Empty board
impl Board {
    pub fn empty() -> Self {
        Self {
            pieces: [Bitboard::EMPTY; Piece::NUM],
            colors: [Bitboard::EMPTY; Color::NUM],
            pc_map: [CPiece::None; Square::NUM],

            stm: Color::White,
            castlingmask: CastlingMask::default(),

            state: BoardState::default(),
            history: Vec::new(),
        }
    }
}

/// Default: Set to start position.
impl Default for Board {
    fn default() -> Self {
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".parse().unwrap()
    }
}

/// Read in a FEN to a board.
/// FEN contains 6 sections:
///
/// 1. Piece placement:
///    e.g "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"
///    p..k : Set corresponding piece here
///    0..8 : This many empty places.
///    /    : Skip to next rank
///
/// 2. Color:
///    w = white, b = black
///
/// 3. Castling rights:
///    See [CastlingRights].
///    This handles both regular and FRC castling.
///
/// 4. En passant:
///    Contains either a square or "-" for none.
///
/// 5. Halfmoves + fullmoves
impl FromStr for Board {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fen = s.split_whitespace().take(6).collect::<Vec<&str>>();
        let mut board = Self::empty();

        // Parse piece placement
        let mut file: u8 = 0;
        let mut rank: u8 = 7;
        for token in fen[0].chars() {
            match token {
                '/' => {
                    if file != 8 {
                        return Err("Invalid piece placement!");
                    }
                    rank = rank.checked_sub(1).ok_or("Invalid rank count!")?;
                    file = 0;
                }
                '1'..='8' => {
                    let empty_squares = token as u8 - b'0';
                    file = file.checked_add(empty_squares).filter(|&f| f <= 8).ok_or("Invalid file count!")?;
                }
                _ => {
                    if file >= 8 {
                        return Err("Too many pieces in rank!");
                    }
                    let piece = CPiece::try_from(token)?;
                    board.set_piece(piece, Square::from(rank * 8 + file));
                    file += 1;
                }
            }
        }

        if rank != 0 || file != 8 {
            return Err("Invalid piece placement!");
        }

        // Parse side to move
        match fen[1] {
            "w" => {
                board.stm = Color::White;
                board.state.key.toggle_color();
            }
            "b" => board.stm = Color::Black,
            _ => return Err("Invalid side to move!"),
        }

        // Parse castling rights
        let (c_rights, c_mask) = match CastlingRights::parse(&board, fen[2]) {
            Ok((r, m)) => (r, m),
            Err(e) => return Err(e),
        };

        board.castlingmask = c_mask;

        board.state.castling = c_rights;

        // Parse en passant
        match fen[3] {
            "-" => board.state.epsq = Square::Invalid,
            s => {
                let epsq: Square = s.parse()?;
                board.state.epsq = epsq;
                board.state.key.toggle_ep(epsq);
            }
        }

        // Parse halfmove count
        board.state.halfmoves = fen[4].parse().map_err(|_| "Invalid halfmove count!")?;

        // Parse fullmove count
        board.state.fullmoves = fen[5].parse().map_err(|_| "Invalid fullmove count!")?;

        Ok(board)
    }
}

/// Board implementations.
impl Board {
    #[inline]
    pub fn pc_bb(&self, c: Color, p: Piece) -> Bitboard {
        self.pieces[p.index()] & self.colors[c.index()]
    }

    /// Gets the position of the king of the given color.
    #[inline]
    pub fn ksq(&self, c: Color) -> Square {
        self.pc_bb(c, Piece::King).lsb()
    }

    /// Get the piece at a given position.
    #[inline]
    pub const fn pc_at(&self, s: Square) -> CPiece {
        self.pc_map[s.index()]
    }

    /// Set the given piece on the given square.
    #[inline]
    pub fn set_piece(&mut self, p: CPiece, s: Square) {
        self.pieces[p.pt().index()].set_bit(s);
        self.colors[p.color().index()].set_bit(s);
        self.pc_map[s.index()] = p;
        self.state.key.toggle_piece(p, s);
    }

    /// Remove the piece on the given square.
    #[inline]
    pub fn pop_piece(&mut self, s: Square) {
        let p = self.pc_at(s);
        self.pieces[p.pt().index()].pop_bit(s);
        self.colors[p.color().index()].pop_bit(s);
        self.pc_map[s.index()] = CPiece::None;
        self.state.key.toggle_piece(p, s);
    }

    /// Gets the piece at the given square.
    #[inline]
    pub fn get_piece(&self, s: Square) -> CPiece {
        self.pc_map[s.index()]
    }
}
