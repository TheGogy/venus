use core::fmt;

use utils::{cfor, rng::next_rng};

use crate::types::{
    castling::CastlingRights,
    color::Color,
    piece::{CPiece, Piece},
    rank_file::File,
    square::Square,
};

pub type Key = u64;

/// Zobrist hash implementation.
/// This is used to get the correct key within the tablebases,
/// as well as some history metrics.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Hash {
    pub key: Key,
    pub pawn_key: Key,
    pub non_pawn_key: [Key; Color::NUM],
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
        self.key ^= COLOR_KEY;
    }

    /// Toggle a piece on a square.
    pub fn toggle_piece(&mut self, p: CPiece, s: Square) {
        let k = PIECE_KEYS[p.idx()][s.idx()];
        self.key ^= k;

        if p.pt() == Piece::Pawn {
            self.pawn_key ^= k;
        } else {
            self.non_pawn_key[p.color().idx()] ^= k;
        }
    }

    /// Toggle castling rights on or off.
    pub const fn toggle_castling(&mut self, cr: CastlingRights) {
        self.key ^= CASTLING_KEYS[cr.idx()];
    }

    /// Toggle en passant on or off for a given square.
    /// If the en passant square is unset, reset ep to zero.
    pub fn toggle_ep(&mut self, epsq: Square) {
        let index = if epsq == Square::Invalid { File::NUM } else { epsq.file().idx() };
        self.key ^= EN_PASSANT_KEYS[index];
    }

    /// Get the main key, adjusted for how close the position is to a 50 move draw.
    pub fn key_adjusted_50mr(&self, halfmoves: usize) -> Key {
        self.key ^ RULE50_KEYS[halfmoves.min(RULE50_KEYS.len() - 1)]
    }
}

/// The bits to toggle on or off for a different color.
pub(crate) static COLOR_KEY: Key = 0x0836_90DB_1CD7_C6C5;

/// The bits to toggle on or off if a given piece is on a given square.
pub(crate) static PIECE_KEYS: [[Key; Square::NUM]; CPiece::NUM] = {
    let mut piece_sq = [[0; Square::NUM]; CPiece::NUM];
    let mut state = 0xDE0D_71DD_0844_AD02;

    cfor!(let mut p = 0; p < CPiece::NUM; p += 1; {
        cfor!(let mut s = 0; s < Square::NUM; s += 1; {
            piece_sq[p][s] = state;
            state = next_rng(state);
        });
    });

    piece_sq
};

/// The bits to toggle on or off when we have some castling rights.
static CASTLING_KEYS: [Key; CastlingRights::NUM] = {
    let mut castling = [0; CastlingRights::NUM];
    let mut state = 0xAC3B_55E2_31CE_6ABB;

    cfor!(let mut i = 0; i < CastlingRights::NUM; i += 1; {
        castling[i] = state;
        state = next_rng(state);
    });

    castling
};

/// The bits to toggle on or off when we have an en passant square on a given file.
/// When EP is unset, this should be zero.
static EN_PASSANT_KEYS: [Key; File::NUM + 1] = {
    let mut en_passant = [0; File::NUM + 1];
    let mut state = 0x3855_0AD0_83D9_4048;

    cfor!(let mut i = 0; i < File::NUM; i += 1; {
        en_passant[i] = state;
        state = next_rng(state);
    });

    en_passant
};

/// The bits to toggle on or off for a given halfmove clock.
static RULE50_KEYS: [Key; 128] = {
    const RULE50_MIN: usize = 14;
    const RULE50_BUCKET: usize = 8;

    let mut keys = [0; 128];
    let mut state = 0x81DE_C436_59CD_8287;

    cfor!(let mut i = RULE50_MIN; i < keys.len(); i += RULE50_BUCKET; {
        state = next_rng(state);
        cfor!(let mut j = 0; j < RULE50_BUCKET && i + j < keys.len(); j += 1; {
            keys[i + j] = state;
        });
    });

    keys
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
