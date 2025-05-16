use core::fmt;
use std::str::FromStr;

use crate::{
    movegen::ALL_MOVE,
    tables::{atk_by_type, leaping_piece::all_pawn_atk},
};

use super::{
    bitboard::Bitboard,
    castling::{CastlingMask, CastlingRights},
    color::Color,
    moves::{Move, MoveFlag},
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
    pub mvp: CPiece,

    // Bitboards for movegen
    pub attacked: Bitboard,
    pub checkers: Bitboard,
    pub pin_diag: Bitboard,
    pub pin_orth: Bitboard,
    pub checkmask: Bitboard,

    // Keys
    pub hash: Hash,
}

/// Contains the current board state.
#[derive(Clone, Debug)]
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

        let mut state = BoardState::default();

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
                    let p = CPiece::try_from(token)?;
                    let s = Square::from_raw(rank * 8 + file);
                    board.set_piece(p, s);
                    state.hash.toggle_piece(p, s);
                    file += 1;
                }
            }
        }

        if rank != 0 || file != 8 {
            return Err("Invalid piece placement!");
        }

        if board.pc_bb(Color::White, Piece::King).nbits() != 1 || board.pc_bb(Color::Black, Piece::King).nbits() != 1 {
            return Err("Incorrect number of kings!");
        }

        // Parse side to move
        match fen[1] {
            "w" => {
                board.stm = Color::White;
                state.hash.toggle_color();
            }
            "b" => board.stm = Color::Black,
            _ => return Err("Invalid side to move!"),
        }

        // Update the board state masks for movegen.
        board.update_masks(&mut state);

        // Parse castling rights
        let (c_rights, c_mask) = match CastlingRights::parse(&board, fen[2]) {
            Ok((r, m)) => (r, m),
            Err(e) => return Err(e),
        };

        board.castlingmask = c_mask;

        state.castling = c_rights;
        state.hash.toggle_castling(c_rights);

        // Parse en passant
        match fen[3] {
            "-" => state.epsq = Square::Invalid,
            s => {
                let epsq: Square = s.parse()?;
                state.epsq = epsq;
                state.hash.toggle_ep(epsq);
            }
        }

        // Parse halfmove count
        state.halfmoves = fen[4].parse().map_err(|_| "Invalid halfmove count!")?;

        // Parse fullmove count
        state.fullmoves = fen[5].parse().map_err(|_| "Invalid fullmove count!")?;

        // Set board state.
        board.state = state;

        Ok(board)
    }
}

/// Translate board from internal representation to FEN.
impl Board {
    /// Get the piece placement in UCI format.
    fn piece_placement_str(&self) -> String {
        let mut fen = String::new();

        for rank in (0..8).rev() {
            let mut empty = 0;

            for file in 0..8 {
                let square = Square::from_raw(rank * 8 + file);
                let piece = self.pc_map[square.index()];

                if piece != CPiece::None {
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                        empty = 0;
                    }
                    fen.push(piece.to_char());
                } else {
                    empty += 1;
                }
            }

            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if rank != 0 {
                fen.push('/');
            }
        }

        fen
    }

    /// Get the whole FEN in UCI format.
    pub fn to_fen(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.piece_placement_str(),
            self.stm,
            self.state.castling.to_str(self),
            if self.state.epsq != Square::Invalid { format!("{}", self.state.epsq) } else { "-".to_string() },
            self.state.halfmoves,
            self.state.fullmoves
        )
    }
}

/// Display a board.
impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut board_str = String::new();
        for rank in (0..8).rev() {
            board_str.push_str(format!("\n {} | ", rank + 1).as_str());

            for file in 0..8 {
                board_str.push(self.pc_map[rank * 8 + file].to_char());
                board_str.push(' ');
            }
        }

        board_str.push_str("\n   +----------------\n     a b c d e f g h");

        write!(
            f,
            "{board_str}
-> FEN : {}
-> Hash: {}
",
            self.to_fen(),
            self.state.hash
        )
    }
}

/// Board implementations.
impl Board {
    /// Get the bitboard of a given piece.
    #[inline]
    pub const fn p_bb(&self, p: Piece) -> Bitboard {
        self.pieces[p.index()]
    }

    /// Get the bitboard of a given color.
    #[inline]
    pub const fn c_bb(&self, c: Color) -> Bitboard {
        self.colors[c.index()]
    }

    /// Get the bitboard of a given piece + color.
    #[inline]
    pub fn pc_bb(&self, c: Color, p: Piece) -> Bitboard {
        self.p_bb(p) & self.c_bb(c)
    }

    /// Get all the diagonal sliders on the board (queens + bishops).
    #[inline]
    pub fn diag_slider(&self, c: Color) -> Bitboard {
        (self.p_bb(Piece::Bishop) | self.p_bb(Piece::Queen)) & self.c_bb(c)
    }

    /// Get all the orthogonal sliders on the board (queens + rooks).
    #[inline]
    pub fn orth_slider(&self, c: Color) -> Bitboard {
        (self.p_bb(Piece::Rook) | self.p_bb(Piece::Queen)) & self.c_bb(c)
    }

    /// Get the total occupancy of the position.
    #[inline]
    pub fn occ(&self) -> Bitboard {
        self.colors[0] | self.colors[1]
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
    pub const fn set_piece(&mut self, p: CPiece, s: Square) {
        self.pieces[p.pt().index()].set_bit(s);
        self.colors[p.color().index()].set_bit(s);
        self.pc_map[s.index()] = p;
    }

    /// Remove the piece on the given square.
    #[inline]
    pub const fn pop_piece(&mut self, s: Square) {
        let p = self.pc_at(s);
        self.pieces[p.pt().index()].pop_bit(s);
        self.colors[p.color().index()].pop_bit(s);
        self.pc_map[s.index()] = CPiece::None;
    }

    /// Gets the piece at the given square.
    #[inline]
    pub const fn get_piece(&self, s: Square) -> CPiece {
        self.pc_map[s.index()]
    }

    /// Find a move given a UCI move string.
    #[inline]
    pub fn find_move(&self, s: &str) -> Option<Move> {
        self.gen_moves::<ALL_MOVE>().iter().find(|&m| m.to_uci(&self.castlingmask) == s).copied()
    }

    /// Get the history at `i` steps back.
    #[inline]
    pub fn hist_sub(&self, i: usize) -> &BoardState {
        &self.history[self.history.len() - i]
    }

    /// Get the current ply of the board.
    #[inline]
    pub fn ply(&self) -> usize {
        self.history.len()
    }

    /// Whether we are in check.
    #[inline]
    pub const fn in_check(&self) -> bool {
        !self.state.checkers.is_empty()
    }

    /// Get the piece that is captured by a move.
    #[inline]
    pub fn captured(&self, m: Move) -> CPiece {
        if m.flag() == MoveFlag::EnPassant {
            CPiece::create(!self.stm, Piece::Pawn)
        } else {
            self.pc_at(m.tgt())
        }
    }

    /// All attacks from a given piece type.
    #[inline]
    pub fn atk_from(&self, p: Piece, c: Color) -> Bitboard {
        match p {
            Piece::Pawn => all_pawn_atk(self.pc_bb(c, p), c),
            _ => {
                let mut atk = Bitboard::EMPTY;
                let pcs = self.pc_bb(c, p);
                let occ = self.occ();

                pcs.bitloop(|s| atk |= atk_by_type(p, s, occ));

                atk
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::board::Board;

    #[test]
    fn test_to_fen() {
        let fens: [&str; 12] = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 10",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            "7B/4PPP1/1n1K2n1/1p6/p1R5/5Pk1/1b2P3/r4b2 w - - 0 1",
            "8/2Q2pKN/3bBp2/p2N3P/6Pp/k3Pp2/7p/8 w - - 0 1",
            "2rkr3/1b1pbppp/1p1q1n2/p1pPp1N1/PnP1P3/1QNB4/1P1BKPPP/3RR3 w - - 6 15",
            "rnbqkbrn/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBRN w GQgq - 0 1",
            "2r1k1r1/2q1bpp1/3p1nn1/p3pb1p/2pP1P2/1P5P/1BPNPNP1/R1RQKBR1 w GCgc - 0 1",
        ];

        for fen in fens {
            let board: Board = fen.parse().unwrap();
            assert_eq!(board.to_fen(), fen);
        }
    }
}
