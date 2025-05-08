use leaping_piece::{king_atk, knight_atk};
use sliding_piece::{bishop_atk, rook_atk};

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
