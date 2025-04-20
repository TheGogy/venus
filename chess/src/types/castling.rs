use std::ops::Not;

use crate::{impl_math_assign_ops, impl_math_ops, tables::sliding_piece::between};

use super::{
    bitboard::Bitboard,
    board::Board,
    color::Color,
    piece::{CPiece, Piece},
    rank_file::{File, Rank},
    square::Square,
};

/// Castling rights.
/// This represents the castling rights for both players.
///
/// Represented as:
/// [wk][bk][wq][bq]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Default)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    /// Total number of possible castling rights.
    pub const NUM: usize = 16;

    /// No castling rights set.
    pub const NONE: Self = Self(0);

    /// All castling rights set.
    pub const ALL: Self = Self(0b1111);

    pub const WK: Self = Self(0b0001); // White kingside
    pub const BK: Self = Self(0b0010); // Black kingside
    pub const WQ: Self = Self(0b0100); // White queenside
    pub const BQ: Self = Self(0b1000); // Black queenside

    // All masks.
    const MASKS: [Self; 4] = [Self::WK, Self::BK, Self::WQ, Self::BQ];

    /// The index of the current castling rights.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The index into the rooks list for these castling rights.
    #[inline]
    pub const fn rook_index(self) -> usize {
        self.0.trailing_zeros() as usize
    }

    /// Whether the given color has kingside castling.
    #[inline]
    pub const fn has_ks(self, c: Color) -> bool {
        self.0 & (0b0001 << c.index() as u8) != 0
    }

    /// Whether the given color has queenside castling.
    #[inline]
    pub const fn has_qs(self, c: Color) -> bool {
        self.0 & (0b0100 << c.index() as u8) != 0
    }

    /// Gets the mask for a given color and side.
    #[inline]
    pub const fn get_mask(c: Color, is_ks: bool) -> Self {
        Self::MASKS[c.index() + !is_ks as usize * 2]
    }
}

// Implement math operations and assignment operations on CastlingRights.
impl_math_ops! {
    CastlingRights,
    BitAnd::bitand,
    BitOr::bitor,
    BitXor::bitxor
}

impl_math_assign_ops! {
    CastlingRights,
    BitAndAssign::bitand_assign,
    BitOrAssign::bitor_assign,
    BitXorAssign::bitxor_assign
}

impl Not for CastlingRights {
    type Output = CastlingRights;

    fn not(self) -> Self {
        unsafe { std::mem::transmute(!self.0) }
    }
}

/// CastlingMask. This allows us to efficiently update the castling rights after a move.
///
/// mask:  The castling mask map that we can use to swap out our castling rights.
/// rooks: The rook starting squares. [ wk , bk , wq , bq ]
pub struct CastlingMask {
    pub mask: [CastlingRights; Square::NUM],
    pub rooks: [Square; 4],
}

/// Default Castling mask. Does not modify castling rights.
impl Default for CastlingMask {
    fn default() -> Self {
        Self { mask: [CastlingRights::ALL; Square::NUM], rooks: [Square::Invalid; 4] }
    }
}

/// CastlingRights implementations.
impl CastlingMask {
    /// Get the mask of the rights to zero out after a move.
    #[inline]
    pub fn zero_out(&self, src: Square, tgt: Square) -> CastlingRights {
        self.mask[src.index()] & self.mask[tgt.index()]
    }

    /// Get the from and to squares for the rook given a king destination.
    /// Assumes that king_to is legal.
    #[inline]
    pub const fn rook_from_to(&self, king_to: Square) -> (Square, Square) {
        match king_to {
            Square::G1 => (self.rooks[0], Square::F1),
            Square::G8 => (self.rooks[1], Square::F8),
            Square::C1 => (self.rooks[2], Square::D1),
            Square::C8 => (self.rooks[3], Square::D8),
            _ => unreachable!(),
        }
    }

    /// Adds rights to the castling mask for a given king and rook square.
    #[inline]
    pub fn add_rights(&mut self, ksq: Square, rsq: Square, r: CastlingRights) {
        self.mask[ksq.index()] &= !r;
        self.mask[rsq.index()] &= !r;
        self.rooks[r.rook_index()] = rsq;
    }

    /// Get the occupancy and attack masks that must be empty.
    #[inline]
    pub fn occ_atk<const KSIDE: bool>(&self, ksq: Square, c: Color) -> (Bitboard, Bitboard) {
        let kt = if KSIDE { Square::G1.relative(c) } else { Square::C1.relative(c) };
        let (rf, rt) = self.rook_from_to(kt);

        // King must not be attacked at any point while moving or at destination.
        let atk = between(ksq, kt) | kt.bb();

        // Neither king or rook should have any piece in their path (except themselves)
        let occ = (atk | between(ksq, rf) | rt.bb()) & !(ksq.bb() | rf.bb());

        (occ, atk)
    }
}

/// UCI castling parsing methods.
///
/// Regular parsing uses the following method:
///
/// K = King      uppercase = White
/// Q = Queen     lowercase = Black
///
/// Fischer Random chess is supported. It uses the following format:
///
/// 1. If the rook used for castling is the closest rook to the side, normal KQkq is used.
///
/// 2. If the rook is NOT the closest to the side, we use the file.
///    Again, uppercase for white, lowercase for black.
///
impl CastlingRights {
    pub fn parse(b: &Board, s: &str) -> Result<(Self, CastlingMask), &'static str> {
        if s == "-" {
            return Ok((Self::NONE, CastlingMask::default()));
        }

        let mut rights = Self::NONE;
        let mut c_mask = CastlingMask::default();

        for token in s.chars() {
            let t = token.to_ascii_uppercase();
            let c = if token.is_ascii_uppercase() { Color::White } else { Color::Black };
            let rook = CPiece::create(c, Piece::Rook);
            let ksq = b.ksq(c);

            let (rsq, mask) = match t {
                'K' => {
                    let mut sq = Square::H1.relative(c);
                    while b.pc_at(sq) != rook {
                        sq = sq.prev();
                    }
                    (sq, CastlingRights::get_mask(c, true))
                }
                'Q' => {
                    let mut sq = Square::A1.relative(c);
                    while b.pc_at(sq) != rook {
                        sq = sq.next();
                    }
                    (sq, CastlingRights::get_mask(c, false))
                }

                'A'..='H' => {
                    let sq = Square::make(Rank::R1.relative(c), File::from(t as u8 - b'A'));
                    (sq, CastlingRights::get_mask(c, ksq < sq))
                }

                _ => return Err("Invalid Castling Rights!"),
            };

            // Add in rights
            c_mask.add_rights(ksq, rsq, mask);
            rights |= mask;
        }

        Ok((rights, c_mask))
    }
}

/// Get a string representing the castling rights.
///
/// This uses the following format:
///
/// 1. If the castling squares are the valid square (wk = H1, wq = A1, etc) then use KQkq.
/// 2. Otherwise, use rook file.
impl CastlingRights {
    pub fn to_str(self, b: &Board) -> String {
        let mut s = String::new();

        if self == Self::NONE {
            s.push('-');
            return s;
        }

        for c in Color::iter() {
            let mut tmp = String::new();
            // Kingside.
            if b.state.castling.has_ks(c) {
                let rook_sq = b.castlingmask.rooks[c.index()];
                tmp.push(if rook_sq == Square::H1.relative(c) { 'K' } else { rook_sq.file().to_char() });
            }

            // Queenside.
            if b.state.castling.has_qs(c) {
                let rook_sq = b.castlingmask.rooks[c.index() + 2];
                tmp.push(if rook_sq == Square::A1.relative(c) { 'Q' } else { rook_sq.file().to_char() });
            }

            // Black is lowercase.
            if c == Color::Black {
                tmp = tmp.to_lowercase();
            }

            s.push_str(&tmp);
        }

        s
    }
}
