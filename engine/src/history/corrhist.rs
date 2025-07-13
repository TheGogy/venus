use chess::{
    Depth,
    types::{color::Color, eval::Eval},
};

use super::HistEntry;

const CH_SIZE: usize = 32768;
const CH_MAX: i32 = 1024;

#[derive(Clone, Debug)]
pub struct CorrHist([[HistEntry; CH_SIZE]; Color::NUM]);

impl Default for CorrHist {
    fn default() -> Self {
        Self([[HistEntry::default(); CH_SIZE]; Color::NUM])
    }
}

impl CorrHist {
    /// The index into this CorrHist.
    const fn idx(key: u64, c: Color) -> (usize, usize) {
        (c.idx(), key as usize % CH_SIZE)
    }

    /// Add a bonus to this CorrHist.
    pub fn add_bonus(&mut self, key: u64, c: Color, bonus: i16) {
        let i = Self::idx(key, c);
        self.0[i.0][i.1].gravity::<CH_MAX>(bonus);
    }

    /// Get a bonus from this CorrHist.
    pub fn get_bonus(&self, key: u64, c: Color) -> i32 {
        let i = Self::idx(key, c);
        self.0[i.0][i.1].0 as i32
    }
}

/// Get the correction bonus for this eval difference at this depth.
pub fn correction_bonus(best: Eval, stat: Eval, depth: Depth) -> i16 {
    const MAX_DIFF: i16 = CH_MAX as i16 / 4;
    ((best.0 - stat.0) as i16 * depth / 8).clamp(-MAX_DIFF, MAX_DIFF)
}
