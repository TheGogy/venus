#![allow(clippy::cast_possible_truncation)]

use chess::{
    tables::{
        leaping_piece::{king_atk, knight_atk, pawn_atk},
        sliding_piece::{bishop_atk, rook_atk},
    },
    types::{bitboard::Bitboard, color::Color, square::Square},
};

// Override Fathom's movegen functions.

#[unsafe(no_mangle)]
pub extern "C" fn ven_tb_knight_attacks(sq: u32) -> u64 {
    knight_atk(Square::from_raw(sq as u8)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn ven_tb_king_attacks(sq: u32) -> u64 {
    king_atk(Square::from_raw(sq as u8)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn ven_tb_rook_attacks(sq: u32, occ: u64) -> u64 {
    rook_atk(Square::from_raw(sq as u8), Bitboard(occ)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn ven_tb_bishop_attacks(sq: u32, occ: u64) -> u64 {
    bishop_atk(Square::from_raw(sq as u8), Bitboard(occ)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn ven_tb_queen_attacks(sq: u32, occ: u64) -> u64 {
    ven_tb_bishop_attacks(sq, occ) | ven_tb_rook_attacks(sq, occ)
}

#[unsafe(no_mangle)]
pub extern "C" fn ven_tb_pawn_attacks(sq: u32, c: bool) -> u64 {
    // NOTE: We represent WHITE = 0, BLACK = 1, Fathom does the opposite.
    pawn_atk(Color::from_raw(!c as u8), Square::from_raw(sq as u8)).0
}
