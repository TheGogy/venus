use core::fmt;

use utils::rng::next_rng;

use super::{
    castling::CastlingRights,
    color::Color,
    piece::{CPiece, Piece},
    rank_file::File,
    square::Square,
};

/// Zobrist hash implementation.
/// This is used to get the correct key within the tablebases,
/// as well as some history metrics.
#[derive(PartialEq, Eq, PartialOrd, Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Hash {
    pub key: u64,
    pub pawn_key: u64,
    pub non_pawn_key: [u64; Color::NUM],
}

/// Print out the Hash.
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.key)
    }
}

impl Hash {
    /// Toggle the color bits after a given move.
    pub const fn toggle_color(&mut self) {
        self.key ^= COLOR_KEY
    }

    /// Toggle a piece on a square.
    pub fn toggle_piece(&mut self, p: CPiece, s: Square) {
        let k = PIECE_KEYS[p.idx()][s.idx()];
        self.key ^= k;

        if p.pt() == Piece::Pawn {
            self.pawn_key ^= k;
        } else {
            self.non_pawn_key[p.color().idx()] ^= k
        }
    }

    /// Toggle castling rights on or off.
    pub const fn toggle_castling(&mut self, cr: CastlingRights) {
        self.key ^= CASTLING_KEYS[cr.idx()]
    }

    /// Toggle en passant on or off for a given square.
    /// If the en passant square is unset, reset ep to zero.
    pub fn toggle_ep(&mut self, epsq: Square) {
        let index = if epsq == Square::Invalid { File::NUM } else { epsq.file().idx() };
        self.key ^= EN_PASSANT_KEYS[index];
    }
}

/// The bits to toggle on or off for a different color.
pub(crate) static COLOR_KEY: u64 = 0x83690DB1CD7C6C5;

/// The bits to toggle on or off if a given piece is on a given square.
pub(crate) static PIECE_KEYS: [[u64; Square::NUM]; CPiece::NUM] = {
    let mut piece_sq = [[0; Square::NUM]; CPiece::NUM];
    let mut state = 0xDE0D71DD0844AD02;

    let mut p = 0;
    while p < CPiece::NUM {
        let mut s = 0;
        while s < Square::NUM {
            piece_sq[p][s] = state;
            state = next_rng(state);
            s += 1;
        }
        p += 1;
    }

    piece_sq
};

/// The bits to toggle on or off when we have some castling rights.
static CASTLING_KEYS: [u64; CastlingRights::NUM] = {
    let mut castling = [0; CastlingRights::NUM];
    let mut state = 0xAC3B55E231CE6ABB;
    let mut i = 0;
    while i < CastlingRights::NUM {
        castling[i] = state;
        state = next_rng(state);
        i += 1;
    }

    castling
};

/// The bits to toggle on or off when we have an en passant square on a given file.
/// When EP is unset, this should be zero.
static EN_PASSANT_KEYS: [u64; File::NUM + 1] = {
    let mut en_passant = [0; File::NUM + 1];
    let mut state = 0x38550AD083D94048;

    let mut i = 0;
    while i < File::NUM {
        en_passant[i] = state;
        state = next_rng(state);
        i += 1;
    }

    en_passant
};

#[cfg(test)]
mod tests {
    use crate::types::{
        board::Board,
        moves::{Move, MoveFlag},
        square::Square,
    };

    #[test]
    fn test_ep_key_diff() {
        let mut b1: Board = "8/2k5/8/8/5p2/8/2K1P1P1/8 w - - 0 1".parse().unwrap();
        b1.make_move(Move::new(Square::E2, Square::E4, MoveFlag::DoublePush));
        b1.make_move(Move::new(Square::C7, Square::C6, MoveFlag::Normal));
        b1.make_move(Move::new(Square::G2, Square::G4, MoveFlag::DoublePush));

        let mut b2: Board = "8/2k5/8/8/5p2/8/2K1P1P1/8 w - - 0 1".parse().unwrap();
        b2.make_move(Move::new(Square::G2, Square::G4, MoveFlag::DoublePush));
        b2.make_move(Move::new(Square::C7, Square::C6, MoveFlag::Normal));
        b2.make_move(Move::new(Square::E2, Square::E4, MoveFlag::DoublePush));

        assert_ne!(b1.state.hash, b2.state.hash);
    }
}
