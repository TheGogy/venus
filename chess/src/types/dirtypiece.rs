use crate::types::{piece::CPiece, square::Square};

/// DirtyPiece enum.
///
/// This represents a feature that we are adding / removing from the NNUE.
#[derive(Clone, Copy, Debug)]
pub enum DirtyPieces {
    /// Normal / Promo / DoublePush.
    /// (+moved dst, -moved src)
    Add1Sub1(PcSq, PcSq),

    /// Capture / En Passant / Capture promo.
    /// (+moved dst, -moved src, -captured src)
    Add1Sub2(PcSq, PcSq, PcSq),

    /// Castling.
    /// (+king dst, +rook dst, -king src, -rook src)
    Add2Sub2(PcSq, PcSq, PcSq, PcSq),

    /// Placeholder for default values. Should never be used.
    None,
}

pub type PcSq = (CPiece, Square);
