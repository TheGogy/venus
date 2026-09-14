use chess::types::{color::Color, piece::CPiece, square::Square};

/// How a perspective reorients the board.
#[derive(Copy, Clone, Debug)]
pub struct Orient {
    pub sq: u8,
    pub pc: u8,
}

impl Orient {
    pub const fn new(ksq: Square, pov: Color) -> Self {
        Self { sq: if pov.to_raw() == Color::Black.to_raw() { 0o70 } else { 0 } | if ksq.is_kingside() { 7 } else { 0 }, pc: pov.to_raw() }
    }

    /// The perspective this reorients to.
    pub const fn pov(self) -> Color {
        Color::from_raw(self.pc)
    }

    /// Re-orient the given square.
    pub const fn sq(self, s: Square) -> u8 {
        s.to_raw() ^ self.sq
    }

    /// Re-orient the given piece.
    pub const fn piece(self, p: CPiece) -> u8 {
        p.to_raw() ^ self.pc
    }

    /// 0 if `c` is us, 1 if it is the opponent.
    pub const fn color(self, c: Color) -> usize {
        (c.to_raw() ^ self.pc) as usize
    }
}
