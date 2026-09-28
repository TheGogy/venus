use chess::movegen::MoveList;
use chess::types::color::Color;
use chess::types::rank_file::File;
use chess::types::{
    board::Board,
    moves::Move,
    piece::{CPiece, Piece},
    square::Square,
};
use utils::memory::boxed_zeroed;

use crate::history::HistEntry;

pub const CAP_HIST_MAX: i32 = 16384;

/// Capture history.
///
/// This is used to record the value of captures during the search,
/// in order to help with move ordering.
#[derive(Clone, Debug)]
pub struct NoisyHist {
    /// We can use [`Piece::NUM`] - 1 because kings cannot be captured by a legal move.
    caps: Box<[[[HistEntry; Piece::NUM - 1]; Square::NUM]; CPiece::NUM]>,
    /// Queen promos are noisy, but they don't capture anything.
    qpromos: Box<[[HistEntry; File::NUM]; Color::NUM]>,
}

// TODO: add tunable history defaults.
impl Default for NoisyHist {
    fn default() -> Self {
        Self { caps: boxed_zeroed(), qpromos: boxed_zeroed() }
    }
}

impl NoisyHist {
    /// Add a bonus to the given move.
    fn add_bonus(&mut self, b: &Board, m: Move, bonus: i16) {
        let (src, dst) = (m.src(), m.dst());
        let pc = b.pc_at(src);
        let cap = b.captured(m).pt();

        if cap == Piece::None {
            self.qpromos[pc.color().idx()][src.file().idx()].gravity::<CAP_HIST_MAX>(bonus);
        } else {
            self.caps[pc.idx()][dst.idx()][cap.idx()].gravity::<CAP_HIST_MAX>(bonus);
        }
    }

    /// Get a bonus for the given move.
    pub fn get_bonus(&self, b: &Board, m: Move) -> i32 {
        let (src, dst) = (m.src(), m.dst());
        let pc = b.pc_at(src);
        let cap = b.captured(m).pt();

        let e = if cap == Piece::None {
            self.qpromos[pc.color().idx()][src.file().idx()].0
        } else {
            self.caps[pc.idx()][dst.idx()][cap.idx()].0
        };

        i32::from(e)
    }

    /// Update the history with the given moves.
    pub fn update(&mut self, b: &Board, best: Move, captures: &MoveList, bonus: i16, malus: i16) {
        for m in captures {
            self.add_bonus(b, *m, -malus);
        }

        if best.flag().is_noisy() {
            self.add_bonus(b, best, bonus);
        }
    }
}
