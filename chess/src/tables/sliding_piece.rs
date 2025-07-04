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
    init_attack_lookups();
}

/// Get all bishop attacks from a square given some occupancy.
pub fn bishop_atk(s: Square, occ: Bitboard) -> Bitboard {
    unsafe { BISHOP_DATA[BISHOP_OFFSET_TABLE[s.idx()].attack_offset(occ)] }
}

/// Get all rook attacks from a square given some occupancy.
pub fn rook_atk(s: Square, occ: Bitboard) -> Bitboard {
    unsafe { ROOK_DATA[ROOK_OFFSET_TABLE[s.idx()].attack_offset(occ)] }
}

/// Get all squares in the line between two squares.
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
struct SquareEntry {
    mask: u64,
    base_idx: usize,

    #[cfg(not(target_arch = "x86_64"))]
    magic: u64,
    #[cfg(not(target_arch = "x86_64"))]
    shift: u64,
}

impl SquareEntry {
    /// Get the attacks for this square entry.
    pub fn attack_offset(&self, occ: Bitboard) -> usize {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::_pext_u64;
            unsafe { self.base_idx + _pext_u64(occ.0, self.mask) as usize }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            self.base_idx + (((occ.0 & self.mask).wrapping_mul(self.magic)) >> self.shift) as usize
        }
    }

    const fn empty() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self { mask: 0, base_idx: 0 }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            Self { mask: 0, base_idx: 0, magic: 0, shift: 0 }
        }
    }
}

const ROOK_TABLE_SIZE: usize = 0x19000;
const BISHOP_TABLE_SIZE: usize = 0x1480;

static mut ROOK_OFFSET_TABLE: [SquareEntry; 64] = [SquareEntry::empty(); 64];
static mut BISHOP_OFFSET_TABLE: [SquareEntry; 64] = [SquareEntry::empty(); 64];
static mut ROOK_DATA: [Bitboard; ROOK_TABLE_SIZE] = [Bitboard(0); ROOK_TABLE_SIZE];
static mut BISHOP_DATA: [Bitboard; BISHOP_TABLE_SIZE] = [Bitboard(0); BISHOP_TABLE_SIZE];

const fn sliding_atk_init(pt: Piece, s: Square, bb: Bitboard) -> Bitboard {
    match pt {
        Piece::Rook => Bitboard(rook_atk_init(s.idx(), bb.0)),
        Piece::Bishop => Bitboard(bishop_atk_init(s.idx(), bb.0)),
        _ => unreachable!(),
    }
}

fn init_attack_table<const N: usize>(
    piece: Piece,
    s: Square,
    pext_table: &mut [SquareEntry; 64],
    table: &mut [Bitboard; N],
    cur_size: &mut usize,
) {
    let edges = Bitboard::edge_mask(s);

    // Initialize the entry
    let entry = &mut pext_table[s.idx()];
    entry.mask = (sliding_atk_init(piece, s, Bitboard::EMPTY) & !edges).0;
    entry.base_idx = *cur_size;

    // Generate the attack table
    let mut occ = Bitboard::EMPTY;
    loop {
        assert!(*cur_size < N);

        table[entry.attack_offset(occ)] = sliding_atk_init(piece, s, occ);
        *cur_size += 1;

        occ.0 = (occ.0.wrapping_sub(entry.mask)) & entry.mask;
        if occ.0 == 0 {
            break;
        }
    }
}

/// Must be run as soon as possible when starting program.
/// This initializes the PEXT lookup tables for all pieces.
#[allow(static_mut_refs)]
pub fn init_attack_lookups() {
    let mut bishop_size = 0;
    let mut rook_size = 0;

    unsafe {
        for s in Square::iter() {
            #[cfg(not(target_arch = "x86_64"))]
            {
                ROOK_OFFSET_TABLE[s.idx()].magic = ROOK_MAGICS[s.idx()];
                ROOK_OFFSET_TABLE[s.idx()].shift = ROOK_SHIFTS[s.idx()];
                BISHOP_OFFSET_TABLE[s.idx()].magic = BISHOP_MAGICS[s.idx()];
                BISHOP_OFFSET_TABLE[s.idx()].shift = BISHOP_SHIFTS[s.idx()];
            }
            init_attack_table(Piece::Rook, s, &mut ROOK_OFFSET_TABLE, &mut ROOK_DATA, &mut rook_size);
            init_attack_table(Piece::Bishop, s, &mut BISHOP_OFFSET_TABLE, &mut BISHOP_DATA, &mut bishop_size);
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
#[rustfmt::skip]
const ROOK_MAGICS: [u64; 64] = [
    0x0a8002c000108020u64,  0x06c00049b0002001u64,  0x0100200010090040u64,  0x02480041000800801u64, 0x0280028004000800u64,  0x0900410008040022u64,  0x0280020001001080u64,   0x02880002041000080u64,
    0x0a000800080400034u64, 0x0004808020004000u64,  0x02290802004801000u64, 0x0411000d00100020u64,  0x00402800800040080u64, 0x000b000401004208u64,  0x02409000100040200u64,  0x0001002100004082u64,
    0x0022878001e24000u64,  0x01090810021004010u64, 0x0801030040200012u64,  0x00500808008001000u64, 0x0a08018014000880u64,  0x08000808004000200u64, 0x0201008080010200u64,   0x0801020000441091u64,
    0x000800080204005u64,   0x01040200040100048u64, 0x000120200402082u64,   0x0d14880480100080u64,  0x012040280080080u64,   0x0100040080020080u64,  0x09020010080800200u64,  0x0813241200148449u64,
    0x0491604001800080u64,  0x0100401000402001u64,  0x04820010021001040u64, 0x00400402202000812u64, 0x0209009005000802u64,  0x0810800601800400u64,  0x04301083214000150u64,  0x0204026458e001401u64,
    0x00040204000808000u64, 0x08001008040010020u64, 0x08410820820420010u64, 0x01003001000090020u64, 0x00804040008008080u64, 0x00012000810020004u64, 0x01000100200040208u64,  0x0430000a044020001u64,
    0x00280009023410300u64, 0x00e0100040002240u64,  0x000200100401700u64,   0x02244100408008080u64, 0x0008000400801980u64,  0x0002000810040200u64,  0x08010100228810400u64,  0x02000009044210200u64,
    0x04080008040102101u64, 0x0040002080411d01u64,  0x02005524060000901u64, 0x0502001008400422u64,  0x0489a000810200402u64, 0x0001004400080a13u64,  0x04000011008020084u64,  0x026002114058042u64,
];

#[cfg(not(target_arch = "x86_64"))]
#[rustfmt::skip]
const BISHOP_MAGICS: [u64; 64] = [
    0x089a1121896040240u64, 0x02004844802002010u64, 0x02068080051921000u64, 0x062880a0220200808u64, 0x0004042004000000u64,  0x0100822020200011u64,  0x0c00444222012000au64,   0x0028808801216001u64,
    0x00400492088408100u64, 0x0201c401040c0084u64,  0x0840800910a0010u64,   0x00082080240060u64,    0x02000840504006000u64, 0x030010c4108405004u64, 0x01008005410080802u64,  0x08144042209100900u64,
    0x00208081020014400u64, 0x004800201208ca00u64,  0x00f18140408012008u64, 0x01004002802102001u64, 0x0841000820080811u64,  0x0040200200a42008u64,  0x0000800054042000u64,   0x088010400410c9000u64,
    0x0520040470104290u64,  0x01004040051500081u64, 0x02002081833080021u64, 0x000400c00c010142u64,  0x0941408200c002000u64, 0x0658810000806011u64,  0x0188071040440a00u64,   0x04800404002011c00u64,
    0x00104442040404200u64, 0x0511080202091021u64,  0x0004022401120400u64,  0x080c0040400080120u64, 0x08040010040820802u64, 0x0480810700020090u64,  0x0102008e00040242u64,   0x0809005202050100u64,
    0x08002024220104080u64, 0x0431008804142000u64,  0x019001802081400u64,   0x0200014208040080u64,  0x03308082008200100u64, 0x041010500040c020u64,  0x04012020c04210308u64,  0x0208220a202004080u64,
    0x0111040120082000u64,  0x06803040141280a00u64, 0x02101004202410000u64, 0x08200000041108022u64, 0x00021082088000u64,    0x002410204010040u64,   0x0040100400809000u64,   0x0822088220820214u64,
    0x0040808090012004u64,  0x000910224040218c9u64, 0x0402814422015008u64,  0x0090014004842410u64,  0x0001000042304105u64,  0x010008830412a00u64,   0x02520081090008908u64,  0x040102000a0a60140u64,
];

#[cfg(not(target_arch = "x86_64"))]
#[rustfmt::skip]
const BISHOP_SHIFTS: [u64; 64] = [
    58, 59, 59, 59, 59, 59, 59, 58,
    59, 59, 59, 59, 59, 59, 59, 59,
    59, 59, 57, 57, 57, 57, 59, 59,
    59, 59, 57, 55, 55, 57, 59, 59,
    59, 59, 57, 55, 55, 57, 59, 59,
    59, 59, 57, 57, 57, 57, 59, 59,
    59, 59, 59, 59, 59, 59, 59, 59,
    58, 59, 59, 59, 59, 59, 59, 58,
];

#[cfg(not(target_arch = "x86_64"))]
#[rustfmt::skip]
const ROOK_SHIFTS: [u64; 64] = [
    52, 53, 53, 53, 53, 53, 53, 52,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    52, 53, 53, 53, 53, 53, 53, 52,
];

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
        blockers.set_bit(Square::E6);
        blockers.set_bit(Square::G4);

        let attacks = rook_atk(pos, blockers);

        assert_bitboard_eq!(attacks, Bitboard(17662768844816));
    }

    #[test]
    fn test_rook_ignore_pieces() {
        let pos = Square::E4;
        let mut blockers = Bitboard::EMPTY;
        blockers.set_bit(Square::B6);
        blockers.set_bit(Square::G7);

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
        blockers.set_bit(Square::G6);
        blockers.set_bit(Square::C2);

        let attacks = bishop_atk(pos, blockers);

        assert_bitboard_eq!(attacks, Bitboard(72695482583368832));
    }

    #[test]
    fn test_bishop_ignore_pieces() {
        let pos = Square::E4;
        let mut blockers = Bitboard::EMPTY;
        blockers.set_bit(Square::B6);
        blockers.set_bit(Square::G7);

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
    fn test_table_initialization() {
        let occ = Square::A3.bb() | Square::C2.bb() | Square::E5.bb() | Square::F2.bb() | Square::F5.bb();

        for s in Square::iter() {
            assert_bitboard_eq!(bishop_atk(s, occ), Bitboard(bishop_atk_init(s.idx(), occ.0)));
            assert_bitboard_eq!(rook_atk(s, occ), Bitboard(rook_atk_init(s.idx(), occ.0)));
        }
    }
}
