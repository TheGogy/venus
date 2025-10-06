use chess::types::{board::Board, moves::Move};
use utils::memory::box_array;

use crate::history::{conthist::PieceTo, movebuffer::MoveBuffer};

use super::HistEntry;

/// Pawn history.
///
/// This is used to note how good the pawn structure is in the position.
#[derive(Clone, Debug)]
pub struct PawnHist(Box<[[HistEntry; PieceTo::NUM]; PAWN_HIST_SIZE]>);

// TODO: add tunable history defaults.
impl Default for PawnHist {
    fn default() -> Self {
        Self(box_array())
    }
}

pub const PAWN_HIST_SIZE: usize = 1024;
pub const PAWN_HIST_MAX: i32 = 8192;

impl PawnHist {
    /// The index into this PawnHist.
    /// [color][from][to]
    const fn idx(b: &Board, m: Move) -> (usize, usize) {
        (b.state.hash.pawn_key as usize % PAWN_HIST_SIZE, PieceTo::from(b, m).idx())
    }

    const fn add_bonus(&mut self, b: &Board, m: Move, bonus: i16) {
        let i = Self::idx(b, m);
        self.0[i.0][i.1].gravity::<PAWN_HIST_MAX>(bonus);
    }

    /// Get a bonus for the given move.
    pub const fn get_bonus(&self, b: &Board, m: Move) -> i32 {
        let i = Self::idx(b, m);
        self.0[i.0][i.1].0 as i32
    }

    /// Update this QuietHist with the given quiet moves.
    pub fn update(&mut self, b: &Board, best: Move, quiets: &MoveBuffer, bonus: i16, malus: i16) {
        for m in quiets {
            self.add_bonus(b, *m, -malus);
        }

        self.add_bonus(b, best, bonus);
    }
}
