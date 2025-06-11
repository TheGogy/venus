use leaping_piece::{king_atk, knight_atk};
use sliding_piece::{BISHOP_ATTACKS, ROOK_ATTACKS, bishop_atk, rook_atk};

use crate::types::{bitboard::Bitboard, piece::Piece, square::Square};

pub mod leaping_piece;
pub mod sliding_piece;

/// Get all attacks of a piece by its type given some occupancy.
#[rustfmt::skip]
pub fn atk_by_type(p: Piece, s: Square, occ: Bitboard) -> Bitboard {
    match p {
        Piece::Pawn | Piece::None => Bitboard::EMPTY,
        Piece::Knight => knight_atk(s),
        Piece::Bishop => bishop_atk(s, occ),
        Piece::Rook   => rook_atk(s, occ),
        Piece::Queen  => bishop_atk(s, occ) | rook_atk(s, occ),
        Piece::King   => king_atk(s),
    }
}

/// Get all attacks of a piece by its type given some occupancy (const).
/// Assumes that occupancy is empty.
#[rustfmt::skip]
pub const fn atk_by_type_const(p: Piece, s: Square) -> Bitboard {
    match p {
        Piece::Pawn | Piece::None => Bitboard::EMPTY,
        Piece::Knight => knight_atk(s),
        Piece::Bishop => BISHOP_ATTACKS[s.idx()],
        Piece::Rook   => ROOK_ATTACKS[s.idx()],
        Piece::Queen  => Bitboard(BISHOP_ATTACKS[s.idx()].0 | ROOK_ATTACKS[s.idx()].0),
        Piece::King   => king_atk(s),
    }
}
