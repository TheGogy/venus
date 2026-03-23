use chess::types::{Depth, color::Color, eval::Eval};
use utils::memory::boxed_zeroed;

use super::HistEntry;

const CORR_HIST_SIZE: usize = 32768;
const CORR_HIST_MAX: i32 = 1024;

/// Correction history.
///
/// This records the difference between what we thought the eval was in static eval, and what it
/// turned out to be through searching the moves. It helps improve the static eval in future
/// searches.
/// <https://www.chessprogramming.org/Static_Evaluation_Correction_History>
#[derive(Clone, Debug)]
pub struct CorrHist(Box<[[HistEntry; CORR_HIST_SIZE]; Color::NUM]>);

impl Default for CorrHist {
    fn default() -> Self {
        Self(boxed_zeroed())
    }
}

impl CorrHist {
    /// The index into this history.
    /// [stm][key]
    #[allow(clippy::cast_possible_truncation)]
    const fn idx(key: u64, c: Color) -> (usize, usize) {
        (c.idx(), key as usize % CORR_HIST_SIZE)
    }

    /// Add a bonus to the given key.
    pub const fn add_bonus(&mut self, key: u64, c: Color, bonus: i16) {
        let i = Self::idx(key, c);
        self.0[i.0][i.1].gravity::<CORR_HIST_MAX>(bonus);
    }

    /// Get a bonus for the given key.
    pub const fn get_bonus(&self, key: u64, c: Color) -> i32 {
        let i = Self::idx(key, c);
        self.0[i.0][i.1].0 as i32
    }
}

/// Get the correction bonus for this eval difference at this depth.
#[allow(clippy::cast_possible_truncation)]
pub fn correction_bonus(best: Eval, stat: Eval, depth: Depth) -> i16 {
    const MAX_DIFF: i32 = CORR_HIST_MAX / 4;
    ((best.0 - stat.0) * depth as i32 / 8).clamp(-MAX_DIFF, MAX_DIFF) as i16
}
