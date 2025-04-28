use leaping_piece::{king_attacks, knight_attacks};
use sliding_piece::{BISHOP_ATTACKS, ROOK_ATTACKS};

use crate::types::{bitboard::Bitboard, piece::Piece, square::Square};

pub mod leaping_piece;
pub mod sliding_piece;

// / Get all attacks of a piece by its type.
pub const fn attacks_by_type(p: Piece, s: Square) -> Bitboard {
    match p {
        Piece::Pawn | Piece::None => Bitboard::EMPTY,
        Piece::Knight => knight_attacks(s),
        Piece::Bishop => BISHOP_ATTACKS[s.index()],
        Piece::Rook => ROOK_ATTACKS[s.index()],
        Piece::Queen => Bitboard(BISHOP_ATTACKS[s.index()].0 | ROOK_ATTACKS[s.index()].0),
        Piece::King => king_attacks(s),
    }
}
