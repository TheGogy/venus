use leaping_piece::{king_attacks, knight_attacks};
use sliding_piece::{bishop_attacks, rook_attacks};

use crate::types::{bitboard::Bitboard, piece::Piece, square::Square};

pub mod leaping_piece;
pub mod sliding_piece;

/// Get all attacks of a piece by its type given some occupancy.
#[rustfmt::skip]
pub fn atk_by_type(p: Piece, s: Square, occ: Bitboard) -> Bitboard {
    match p {
        Piece::Pawn | Piece::None => Bitboard::EMPTY,
        Piece::Knight => knight_attacks(s),
        Piece::Bishop => bishop_attacks(s, occ),
        Piece::Rook   => rook_attacks(s, occ),
        Piece::Queen  => bishop_attacks(s, occ) | rook_attacks(s, occ),
        Piece::King   => king_attacks(s),
    }
}
