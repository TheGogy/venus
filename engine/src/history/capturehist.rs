use chess::types::{
    board::Board,
    moves::Move,
    piece::{CPiece, Piece},
    square::Square,
};

use super::{HistEntry, movebuffer::MoveBuffer};

pub const CAP_HIST_MAX: i32 = 16384;

/// Capture history.
///
/// This is used to record the value of captures during the search,
/// in order to help with move ordering.
/// We can use Piece::NUM - 1 because kings cannot be captured by a legal move.
#[derive(Clone, Debug)]
pub struct CaptureHist([[[HistEntry; Piece::NUM - 1]; Square::NUM]; CPiece::NUM]);

// TODO: add tunable history defaults.
impl Default for CaptureHist {
    fn default() -> Self {
        Self([[[HistEntry::default(); Piece::NUM - 1]; Square::NUM]; CPiece::NUM])
    }
}

impl CaptureHist {
    /// The index into this NoisyHist.
    /// [piecetype][captured][to]
    fn idx(b: &Board, m: Move) -> (usize, usize, usize) {
        (b.pc_at(m.src()).idx(), m.dst().idx(), b.captured(m).pt().idx())
    }

    /// Add a bonus to the given move.
    fn add_bonus(&mut self, b: &Board, m: Move, bonus: i16) {
        let i = Self::idx(b, m);
        self.0[i.0][i.1][i.2].gravity::<CAP_HIST_MAX>(bonus);
    }

    /// Get a bonus for the given move.
    pub fn get_bonus(&self, b: &Board, m: Move) -> i32 {
        let i = Self::idx(b, m);
        self.0[i.0][i.1][i.2].0 as i32
    }

    /// Update the NoisyHist with the given moves.
    pub fn update(&mut self, b: &Board, best: Move, captures: &MoveBuffer, bonus: i16, malus: i16) {
        for m in captures {
            self.add_bonus(b, *m, -malus);
        }

        if best.flag().is_cap() {
            self.add_bonus(b, best, bonus);
        }
    }
}
