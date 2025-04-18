use crate::types::{bitboard::Bitboard, color::Color, direction::Direction, square::Square};

/// Get pawn attacks for a given color and square.
pub const fn pawn_attacks(c: Color, s: Square) -> Bitboard {
    PAWN_DATA[c.index()][s.index()]
}

/// Get all knight attacks for a given square.
pub const fn knight_attacks(s: Square) -> Bitboard {
    KNIGHT_DATA[s.index()]
}

/// Get all king attacks for a given square.
pub const fn king_attacks(s: Square) -> Bitboard {
    KING_DATA[s.index()]
}

/// Get all pawn attacks for a given side.
pub const fn all_pawn_atk(bb: Bitboard, c: Color) -> Bitboard {
    match c {
        Color::White => Bitboard(bb.shift(Direction::NorthEast).0 | bb.shift(Direction::NorthWest).0),
        Color::Black => Bitboard(bb.shift(Direction::SouthEast).0 | bb.shift(Direction::SouthWest).0),
    }
}

/// Pawn attacks bitboard lookup table.
const PAWN_DATA: [[Bitboard; 64]; 2] = {
    let mut pd = [[Bitboard(0); 64]; 2];
    let mut sq = 0;

    while sq < Square::NUM {
        let pawn = Bitboard(1u64 << sq);
        pd[0][sq] = all_pawn_atk(pawn, Color::White);
        pd[1][sq] = all_pawn_atk(pawn, Color::Black);
        sq += 1;
    }

    pd
};

/// Initializes the knight attack table. Do not call at runtime.
#[rustfmt::skip]
const fn init_knight_atk(bb: Bitboard) -> Bitboard {
    Bitboard(
        bb.shift(Direction::NorthEast).shift(Direction::North).0
      | bb.shift(Direction::NorthWest).shift(Direction::North).0
      | bb.shift(Direction::NorthEast).shift(Direction::East).0
      | bb.shift(Direction::SouthEast).shift(Direction::East).0
      | bb.shift(Direction::SouthEast).shift(Direction::South).0
      | bb.shift(Direction::SouthWest).shift(Direction::South).0
      | bb.shift(Direction::SouthWest).shift(Direction::West).0
      | bb.shift(Direction::NorthWest).shift(Direction::West).0,
    )
}

/// Knight attacks bitboard lookup table.
const KNIGHT_DATA: [Bitboard; 64] = {
    let mut attacks = [Bitboard(0); 64];
    let mut square = 0;

    while square < 64 {
        let knight = Bitboard(1u64 << square);
        attacks[square] = init_knight_atk(knight);
        square += 1;
    }

    attacks
};

/// Initializes the king attack table. Do not call at runtime.
#[rustfmt::skip]
const fn init_king_atk(bb: Bitboard) -> Bitboard {
    Bitboard(
        bb.shift(Direction::North).0
      | bb.shift(Direction::East).0
      | bb.shift(Direction::South).0
      | bb.shift(Direction::West).0
      | bb.shift(Direction::NorthEast).0
      | bb.shift(Direction::NorthWest).0
      | bb.shift(Direction::SouthEast).0
      | bb.shift(Direction::SouthWest).0,
    )
}

/// King attacks bitboard lookup table.
const KING_DATA: [Bitboard; 64] = {
    let mut attacks = [Bitboard(0); 64];
    let mut square = 0;

    while square < 64 {
        let king = Bitboard(1u64 << square);
        attacks[square] = init_king_atk(king);
        square += 1;
    }

    attacks
};
