use crate::impl_from_type;

use super::{piece::Piece, square::Square};

/// Moves (encoded as u16)
/// bits  0 - 5  : from square
/// bits  6 - 11 : to square
/// bits 12 - 15 : Move flag
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Default, Hash)]
pub struct Move(pub u16);

impl Move {
    /// Construct a move.
    #[inline]
    pub const fn new(src: Square, tgt: Square, flag: MoveFlag) -> Self {
        Move((src as u16) << 6 | (tgt as u16) | (flag as u16) << 12)
    }

    /// Gets the source square of this move.
    #[inline]
    pub fn src(self) -> Square {
        Square::from(((self.0 >> 6) & 63) as u8)
    }

    /// Gets the target square of this move.
    #[inline]
    pub fn tgt(self) -> Square {
        Square::from((self.0 & 63) as u8)
    }

    /// Gets the flag of this move.
    #[inline]
    pub fn flag(self) -> MoveFlag {
        const TYPE: u16 = 0xF << 12;
        MoveFlag::from(((self.0 & TYPE) >> 12) as u8)
    }
}

/// MoveFlag. Shows the type of move.
#[rustfmt::skip]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
#[repr(u8)]
pub enum MoveFlag {
    // Quiet moves have first two bits unset.
    Normal     = 0b0000,
    DoublePush = 0b0001,
    Castling   = 0b0010,

    // Captures have third bit set.
    Capture    = 0b0100,
    EnPassant  = 0b0101,

    // Promotions have fourth bit set.
    // The promoting piece is the last 2 bits - Knight.
    PromoN     = 0b1000,
    PromoB     = 0b1001,
    PromoR     = 0b1010,
    PromoQ     = 0b1011,

    // Capture promotions have both bits set.
    CPromoN     = 0b1100,
    CPromoB     = 0b1101,
    CPromoR     = 0b1110,
    CPromoQ     = 0b1111,
}

impl MoveFlag {
    /// Whether this MoveFlag denotes a capture.
    #[inline]
    pub const fn is_cap(self) -> bool {
        self as u16 & 0b0100 != 0
    }

    /// Whether this MoveFlag denotes a promotion.
    /// This includes underpromotions.
    #[inline]
    pub const fn is_promo(self) -> bool {
        self as u16 & 0b1000 != 0
    }

    /// Whether this MoveFlag denotes a quiet move.
    #[inline]
    pub const fn is_quiet(self) -> bool {
        self as u16 & 0b1100 == 0
    }

    /// Whether this MoveFlag denotes a promotion that is not a queen.
    #[inline]
    pub const fn is_underpromo(self) -> bool {
        self as u16 & 0b1011 != 0b1011
    }

    /// Get the piece this MoveFlag denotes a promotion to.
    /// Equivalent to the last 2 bits plus a knight (hence the +1).
    #[inline]
    pub const fn get_promo(self) -> Piece {
        unsafe { std::mem::transmute(((self as u8) & 0b0011) + 1) }
    }
}

impl_from_type! {
    MoveFlag, u8,
    u8,
    u16,
    u32,
    u64,
    i16,
    i32,
    i64,
    usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_creation() {
        let m = Move::new(Square::E2, Square::E4, MoveFlag::Normal);
        assert_eq!(m.src(), Square::E2);
        assert_eq!(m.tgt(), Square::E4);
        assert_eq!(m.flag(), MoveFlag::Normal);
    }

    #[test]
    fn test_move_flag() {
        assert!(MoveFlag::Normal.is_quiet());
        assert!(MoveFlag::CPromoB.is_cap());
        assert!(MoveFlag::Castling.is_quiet());
        assert!(MoveFlag::PromoQ.is_promo());
        assert!(MoveFlag::PromoR.is_underpromo());
    }
}
