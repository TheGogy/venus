use chess::types::{board::Board, moves::Move, piece::CPiece, square::Square};
use utils::memory::box_array;

use super::{HistEntry, movebuffer::MoveBuffer};

const CONT_HIST_MAX: i32 = 16384;
pub const CONT_NUM: usize = 2;

/// Continuation history.
///
/// This records the history of combinations of the current move and the move `n` plies ago.
#[derive(Clone, Debug)]
pub struct ContHist(Box<[[[HistEntry; Square::NUM]; Square::NUM]; PieceTo::NUM]>);

// TODO: add tunable history defaults.
impl Default for ContHist {
    fn default() -> Self {
        Self(box_array())
    }
}

impl ContHist {
    /// The index into this ContHist.
    /// [pieceto][from][to]
    const fn idx(m: Move, pt: PieceTo) -> (usize, usize, usize) {
        (pt.idx(), m.src().idx(), m.dst().idx())
    }

    /// Add a bonus to the given move pair.
    const fn add_bonus(&mut self, m: Move, pt: PieceTo, bonus: i16) {
        let i = Self::idx(m, pt);
        self.0[i.0][i.1][i.2].gravity::<CONT_HIST_MAX>(bonus);
    }

    /// Get a bonus from the given move pair.
    pub const fn get_bonus(&self, m: Move, pt: PieceTo) -> i32 {
        let i = Self::idx(m, pt);
        self.0[i.0][i.1][i.2].0 as i32
    }

    /// Update this ContHist with the new best move.
    pub fn update(&mut self, best: Move, pt: PieceTo, quiets: &MoveBuffer, bonus: i16, malus: i16) {
        self.add_bonus(best, pt, bonus);

        for m in quiets {
            self.add_bonus(*m, pt, -malus);
        }
    }
}

// PieceTo.
// A helper type that allows us to index into ContHists more easily.
#[derive(Debug, Clone, Copy)]
pub struct PieceTo(usize);

impl PieceTo {
    pub const NUM: usize = CPiece::NUM * Square::NUM;

    /// The index of this PieceTo.
    pub const fn idx(self) -> usize {
        self.0
    }

    /// Construct a PieceTo from a piece and a move.
    pub const fn from(b: &Board, m: Move) -> Self {
        let p = b.pc_at(m.src());
        Self(p.idx() * Square::NUM + m.dst().idx())
    }
}
