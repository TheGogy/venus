use ctor::ctor;

use crate::types::{
    bitboard::Bitboard,
    direction::{Direction, sliding_ray},
    piece::Piece,
    square::Square,
};

/// Runs before main.
#[ctor]
fn init() {
    init_pext_lookups();
}

/// Get all bishop attacks from a square given some occupancy.
#[inline]
pub fn bishop_atk(s: Square, occ: Bitboard) -> Bitboard {
    unsafe { BISHOP_PEXT_TABLE[s.idx()].attacks(occ) }
}

/// Get all rook attacks from a square given some occupancy.
#[inline]
pub fn rook_atk(s: Square, occ: Bitboard) -> Bitboard {
    unsafe { ROOK_PEXT_TABLE[s.idx()].attacks(occ) }
}

/// Get all squares in the line between two squares.
#[inline]
pub const fn between(a: Square, b: Square) -> Bitboard {
    BETWEEN_TABLE[a.idx()][b.idx()]
}

/// Initialize the bishop attacks.
#[rustfmt::skip]
const fn bishop_atk_init(s: usize, occ: u64) -> u64 {
    sliding_ray(Direction::NorthEast, s, occ).0
  | sliding_ray(Direction::NorthWest, s, occ).0
  | sliding_ray(Direction::SouthEast, s, occ).0
  | sliding_ray(Direction::SouthWest, s, occ).0
}

/// Raw bishop attacks table. This does not take blockers into account.
pub static BISHOP_ATTACKS: [Bitboard; 64] = {
    let mut attacks = [Bitboard(0); 64];
    let mut square = 0;

    while square < 64 {
        attacks[square] = Bitboard(bishop_atk_init(square, 0));
        square += 1;
    }

    attacks
};

/// Initialize the rook attacks.
#[rustfmt::skip]
const fn rook_atk_init(s: usize, occ: u64) -> u64 {
    sliding_ray(Direction::North, s, occ).0
  | sliding_ray(Direction::South, s, occ).0
  | sliding_ray(Direction::East,  s, occ).0
  | sliding_ray(Direction::West,  s, occ).0
}

/// Raw rook attacks table. This does not take blockers into account.
pub static ROOK_ATTACKS: [Bitboard; 64] = {
    let mut attacks = [Bitboard(0); 64];
    let mut square = 0;

    while square < 64 {
        attacks[square] = Bitboard(rook_atk_init(square, 0));
        square += 1;
    }

    attacks
};

/// Table of the line between two squares.
#[allow(clippy::large_const_arrays)]
pub static BETWEEN_TABLE: [[Bitboard; 64]; 64] = {
    let mut between = [[Bitboard(0); 64]; 64];

    let mut s1 = 0;

    while s1 < 64 {
        let mut s2 = 0;
        while s2 < 64 {
            let bb1 = 1u64 << s1;
            let bb2 = 1u64 << s2;

            if BISHOP_ATTACKS[s1].0 & bb2 != 0 {
                between[s1][s2] = Bitboard(bishop_atk_init(s1, bb2) & bishop_atk_init(s2, bb1));
            } else if ROOK_ATTACKS[s1].0 & bb2 != 0 {
                between[s1][s2] = Bitboard(rook_atk_init(s1, bb2) & rook_atk_init(s2, bb1));
            }
            s2 += 1;
        }
        s1 += 1;
    }

    between
};

/// Entry within pext lookup table.
#[derive(Clone, Copy)]
struct PextEntry {
    mask: Bitboard,
    data: *mut Bitboard,
}

impl PextEntry {
    /// Get the attacks for this PEXT entry.
    #[inline]
    pub fn attacks(&self, occ: Bitboard) -> Bitboard {
        // Safety: The data pointer is valid and points to the static arrays.
        unsafe { *self.data.add(pext(occ.0, self.mask.0) as usize) }
    }

    const fn empty() -> Self {
        Self { mask: Bitboard(0), data: std::ptr::null_mut() }
    }
}

/// Parallel bit extract wrapper.
#[inline]
fn pext(a: u64, b: u64) -> u64 {
    use std::arch::x86_64::_pext_u64;
    unsafe { _pext_u64(a, b) }
}

const ROOK_TABLE_SIZE: usize = 0x19000;
const BISHOP_TABLE_SIZE: usize = 0x1480;

static mut ROOK_PEXT_TABLE: [PextEntry; 64] = [PextEntry::empty(); 64];
static mut BISHOP_PEXT_TABLE: [PextEntry; 64] = [PextEntry::empty(); 64];
static mut ROOK_DATA: [Bitboard; ROOK_TABLE_SIZE] = [Bitboard(0); ROOK_TABLE_SIZE];
static mut BISHOP_DATA: [Bitboard; BISHOP_TABLE_SIZE] = [Bitboard(0); BISHOP_TABLE_SIZE];

const fn sliding_atk_init(pt: Piece, s: Square, bb: Bitboard) -> Bitboard {
    match pt {
        Piece::Rook => Bitboard(rook_atk_init(s.idx(), bb.0)),
        Piece::Bishop => Bitboard(bishop_atk_init(s.idx(), bb.0)),
        _ => unreachable!(),
    }
}

fn init_pext_table<const N: usize>(
    piece: Piece,
    s: Square,
    pext_table: &mut [PextEntry; 64],
    data: &mut [Bitboard; N],
    prev_size: &mut usize,
) -> usize {
    let edges = Bitboard::edge_mask(s);

    // Calculate the data pointer offset
    let data_ptr = if s == Square::A1 {
        data.as_mut_ptr()
    } else {
        unsafe {
            let prev_entry = &pext_table[s.idx() - 1];
            prev_entry.data.add(*prev_size)
        }
    };

    // Initialize the entry
    let entry = &mut pext_table[s.idx()];
    entry.mask = sliding_atk_init(piece, s, Bitboard::EMPTY) & !edges;
    entry.data = data_ptr;

    let mut size = 0;

    // Generate the attack table
    let mut occ = Bitboard::EMPTY;
    loop {
        unsafe {
            *entry.data.add(pext(occ.0, entry.mask.0) as usize) = sliding_atk_init(piece, s, occ);
        }

        size += 1;

        occ.0 = (occ.0.wrapping_sub(entry.mask.0)) & entry.mask.0;
        if occ.0 == 0 {
            break;
        }
    }

    *prev_size = size;
    size
}

/// Must be run as soon as possible when starting program.
/// This initializes the PEXT lookup tables for all pieces.
#[allow(static_mut_refs)]
pub fn init_pext_lookups() {
    let mut bishop_size = 0;
    let mut rook_size = 0;

    unsafe {
        for s in Square::iter() {
            init_pext_table(Piece::Bishop, s, &mut BISHOP_PEXT_TABLE, &mut BISHOP_DATA, &mut bishop_size);
            init_pext_table(Piece::Rook, s, &mut ROOK_PEXT_TABLE, &mut ROOK_DATA, &mut rook_size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_bitboard_eq;

    #[test]
    fn test_rook_basic_movements() {
        // Test rook in the center
        let pos = Square::E4;
        let attacks = rook_atk(pos, Bitboard::EMPTY);

        assert_bitboard_eq!(attacks, Bitboard(1157442769150545936));
    }

    #[test]
    fn test_rook_edge_cases() {
        // Test rook on corner
        let corner = Square::A1;
        let attacks = rook_atk(corner, Bitboard::EMPTY);
        assert_bitboard_eq!(attacks, Bitboard(72340172838076926));
    }

    #[test]
    fn test_rook_blocking_pieces() {
        let pos = Square::E4;
        let mut blockers = Bitboard::EMPTY;
        blockers.set_bit(Square::E6); // Blocker two squares up
        blockers.set_bit(Square::G4); // Blocker two squares right

        let attacks = rook_atk(pos, blockers);

        assert_bitboard_eq!(attacks, Bitboard(17662768844816));
    }

    #[test]
    fn test_rook_ignore_pieces() {
        let pos = Square::E4;
        let mut blockers = Bitboard::EMPTY;
        blockers.set_bit(Square::B6); // Random square
        blockers.set_bit(Square::G7); // Random square

        let attacks = rook_atk(pos, blockers);

        assert_bitboard_eq!(attacks, Bitboard(1157442769150545936));
    }

    #[test]
    fn test_bishop_basic_movements() {
        let pos = Square::E4;
        let attacks = bishop_atk(pos, Bitboard::EMPTY);

        assert_bitboard_eq!(attacks, Bitboard(108724279602332802));
    }

    #[test]
    fn test_bishop_edge_cases() {
        // Test bishop on corner
        let corner = Square::H1;
        let attacks = bishop_atk(corner, Bitboard::EMPTY);

        assert_bitboard_eq!(attacks, Bitboard(72624976668147712));
    }

    #[test]
    fn test_bishop_blocking_pieces() {
        let pos = Square::E4;
        let mut blockers = Bitboard::EMPTY;
        blockers.set_bit(Square::G6); // Blocker two squares up-right
        blockers.set_bit(Square::C2); // Blocker two squares down-left

        let attacks = bishop_atk(pos, blockers);

        assert_bitboard_eq!(attacks, Bitboard(72695482583368832));
    }

    #[test]
    fn test_bishop_ignore_pieces() {
        let pos = Square::E4;
        let mut blockers = Bitboard::EMPTY;
        blockers.set_bit(Square::B6); // Random square
        blockers.set_bit(Square::G7); // Random square

        let attacks = bishop_atk(pos, blockers);

        assert_bitboard_eq!(attacks, Bitboard(108724279602332802));
    }

    #[test]
    fn test_between_table() {
        // Test horizontal between
        let b = between(Square::A1, Square::H1);
        assert_bitboard_eq!(b, Bitboard(126));

        // Test diagonal between
        let b = between(Square::A1, Square::H8);
        assert_bitboard_eq!(b, Bitboard(18049651735527936));

        // Test short distance
        let b = between(Square::E1, Square::B4);
        assert_bitboard_eq!(b, Bitboard(264192));
    }

    #[test]
    fn test_pext_initialization() {
        unsafe {
            // Test that tables are properly initialized
            // And that masks are properly set
            for r in ROOK_PEXT_TABLE {
                assert!(!r.data.is_null());
                assert!(r.mask.0 != 0);
            }
            for b in BISHOP_PEXT_TABLE {
                assert!(!b.data.is_null());
                assert!(b.mask.0 != 0);
            }
        }
    }
}
