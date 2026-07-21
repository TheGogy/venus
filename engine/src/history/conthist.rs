use chess::{
    movegen::MoveList,
    types::{board::Board, moves::Move, piece::CPiece, square::Square},
};
use utils::memory::boxed_zeroed;

use crate::history::HistEntry;

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
        Self(boxed_zeroed())
    }
}

impl ContHist {
    /// The index into this history.
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

    /// Update the history with the given moves.
    pub fn update(&mut self, best: Move, pt: PieceTo, other_moves: &MoveList, bonus: i16, malus: i16) {
        self.add_bonus(best, pt, bonus);

        for m in other_moves {
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

    /// The index of this [`PieceTo`].
    pub const fn idx(self) -> usize {
        assert!(self.0 < PieceTo::NUM);
        self.0
    }

    /// Construct a [`PieceTo`] from a piece and a move.
    /// NOTE: (this should be used *after* the move has been made on the board.)
    pub const fn from(b: &Board, m: Move) -> Self {
        let s = m.src();
        let p = b.pc_at(s);
        Self(p.idx() * Square::NUM + s.idx())
    }
}
