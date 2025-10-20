use crate::impl_from_type;

use super::{castling::CastlingMask, piece::Piece, square::Square};

/// Moves (encoded as u16)
/// bits  0 - 5  : from square
/// bits  6 - 11 : to square
/// bits 12 - 15 : Move flag
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Default, Hash)]
#[repr(transparent)]
pub struct Move(pub u16);

impl Move {
    /// None.
    /// This represents a move that is yet to be populated.
    pub const NONE: Self = Self(0x00);

    /// Construct a move.
    pub const fn new(src: Square, dst: Square, flag: MoveFlag) -> Self {
        Move((src as u16) << 6 | (dst as u16) | (flag as u16) << 12)
    }

    /// Gets the source square of this move.
    pub const fn src(self) -> Square {
        Square::from_raw(((self.0 >> 6) & 63) as u8)
    }

    /// Gets the destination square of this move.
    pub const fn dst(self) -> Square {
        Square::from_raw((self.0 & 63) as u8)
    }

    /// Gets the flag of this move.
    pub const fn flag(self) -> MoveFlag {
        const TYPE: u16 = 0xF << 12;
        MoveFlag::from_raw(((self.0 & TYPE) >> 12) as u8)
    }

    /// Whether the move is none.
    pub const fn is_none(&self) -> bool {
        self.0 == 0x00
    }

    /// Returns self if valid, otherwise evaluates `f` and returns the result.
    pub fn is_some_or<F: FnOnce() -> Move>(self, f: F) -> Move {
        if self.is_none() { f() } else { self }
    }

    /// Display the move according to UCI format.
    pub fn to_uci(self, cm: &CastlingMask) -> String {
        // Invalid moves.
        if self.is_none() {
            return "0000".to_owned();
        }

        let flag = self.flag();

        // Promotions.
        if flag.is_promo() {
            return format!("{}{}{}", self.src(), self.dst(), flag.get_promo().to_char());
        }

        // Castling.
        // In regular -> Denoted by (king from, king to) - and so handled same as other moves.
        // In FRC     -> Denoted by the king moving onto the rook square.
        if cm.frc && flag == MoveFlag::Castling {
            let (rf, _) = cm.rook_src_dst(self.dst());
            return format!("{}{}", self.src(), rf);
        }

        // All other moves are just <from, to>.
        format!("{}{}", self.src(), self.dst())
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
    // The promoting piece is the last 2 bits + Knight.
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
    pub const fn is_cap(self) -> bool {
        self as u16 & 0b0100 != 0
    }

    /// Whether this MoveFlag denotes a promotion.
    /// This includes underpromotions.
    pub const fn is_promo(self) -> bool {
        self as u16 & 0b1000 != 0
    }

    /// Whether this MoveFlag denotes a quiet promotion.
    /// This includes underpromotions.
    pub const fn is_qpromo(self) -> bool {
        self.is_promo() && !self.is_cap()
    }

    /// Whether this MoveFlag denotes a quiet move.
    pub const fn is_quiet(self) -> bool {
        self as u16 & 0b1100 == 0
    }

    /// Whether this MoveFlag denotes a noisy move. (i.e capture or queen promo)
    pub const fn is_noisy(self) -> bool {
        !self.is_quiet() || self as u16 == 0b1011
    }

    /// Whether this MoveFlag denotes a promotion that is not a queen.
    pub const fn is_underpromo(self) -> bool {
        self.is_promo() && self as u16 & 0b1011 != 0b1011
    }

    /// Get the piece this MoveFlag denotes a promotion to.
    /// Equivalent to the last 2 bits plus a knight (hence the +1).
    pub const fn get_promo(self) -> Piece {
        unsafe { std::mem::transmute(((self as u8) & 0b0011) + 1) }
    }
}

impl_from_type! {
    MoveFlag, u8, 16,
    [i64, i32, i16, i8, u64, u32, u16, u8, usize]
}
