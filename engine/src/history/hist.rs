use chess::types::{board::Board, moves::Move};

use crate::tunables::params::tunables::*;

use super::{movebuffer::MoveBuffer, noisyhist::NoisyHist, quiethist::QuietHist};

#[derive(Clone, Debug, Default)]
pub struct History {
    pub quiet: QuietHist,
    pub noisy: NoisyHist,
}

impl History {
    /// Get the bonus and malus for history entries at a given depth.
    #[inline]
    fn hist_delta(depth: usize) -> (i16, i16) {
        let d = depth as i16;

        let b = hist_bonus_max().min(d * hist_bonus_mult() - hist_bonus_base());
        let m = hist_malus_max().min(d * hist_malus_mult() - hist_malus_base());

        (b, m)
    }

    /// Update the history tables based on the moves played.
    pub fn update(&mut self, b: &Board, best: Move, depth: usize, quiets: &MoveBuffer, noisies: &MoveBuffer) {
        let (bonus, malus) = Self::hist_delta(depth);

        self.noisy.update(b, best, noisies, bonus, malus);

        if best.flag().is_quiet() {
            self.quiet.update(b, best, quiets, bonus, malus);
        }
    }
}
