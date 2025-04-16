use super::{
    bitboard::Bitboard,
    castling::{CastlingMask, CastlingRights},
    color::Color,
    piece::{CPiece, Piece},
    square::Square,
    zobrist::Hash,
};

/// Contains the current board state.
pub struct Board {
    // Piece placement
    pub pieces: [Bitboard; Piece::NUM],
    pub colors: [Bitboard; Color::NUM],
    pub pc_map: [CPiece; Square::NUM],

    // Game state
    pub stm: Color,
    pub epsq: Square,
    pub halfmoves: usize,
    pub fullmoves: usize,
    pub castling: CastlingRights,
    pub hash: Hash,
}

/// Empty board
impl Board {
    pub fn empty() -> Self {
        Self {
            pieces: [Bitboard::EMPTY; Piece::NUM],
            colors: [Bitboard::EMPTY; Color::NUM],
            pc_map: [CPiece::None; Square::NUM],

            stm: Color::White,
            epsq: Square::Invalid,
            halfmoves: 0,
            fullmoves: 0,
            castling: CastlingRights::NONE,
            hash: Hash::default(),
        }
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
/// 5.
///
impl Board {
    pub fn parse(s: &str) -> Result<(Self, CastlingMask), &'static str> {
        let fen = s.split_whitespace().take(6).collect::<Vec<&str>>();
        let mut board = Self::empty();

        // Parse piece placement
        let mut file: u8 = 0;
        let mut rank: u8 = 0;
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
                    file = file
                        .checked_add(empty_squares)
                        .filter(|&f| f <= 8)
                        .ok_or("Invalid file count!")?;
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
        match fen[2] {
            "w" => {
                board.stm = Color::White;
                board.hash.toggle_color();
            }
            "b" => board.stm = Color::Black,
            _ => return Err("Invalid side to move!"),
        }

        // Parse castling rights
        let (c_rights, c_mask) = match CastlingRights::parse(&board, fen[3]) {
            Ok((r, m)) => (r, m),
            Err(e) => return Err(e),
        };

        board.castling = c_rights;

        // Parse en passant
        match fen[3] {
            "-" => board.epsq = Square::Invalid,
            s => {
                let epsq: Square = s.parse()?;
                board.epsq = epsq;
                board.hash.toggle_ep(epsq);
            }
        }

        // Parse halfmove count
        board.halfmoves = fen[4].parse().map_err(|_| "Invalid halfmove count!")?;

        // Parse fullmove count
        board.fullmoves = fen[5].parse().map_err(|_| "Invalid fullmove count!")?;

        Ok((board, c_mask))
    }
}

/// Board implementations.
impl Board {
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
        self.hash.toggle_piece(p, s);
    }

    /// Remove the piece on the given square.
    #[inline]
    pub fn pop_piece(&mut self, s: Square) {
        let p = self.pc_at(s);
        self.pieces[p.pt().index()].pop_bit(s);
        self.colors[p.color().index()].pop_bit(s);
        self.pc_map[s.index()] = CPiece::None;
        self.hash.toggle_piece(p, s);
    }
}
