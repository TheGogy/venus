pub mod conthist;
pub mod corrhist;
pub mod noisyhist;
pub mod quiethist;

use chess::types::Depth;

use crate::tunables::params::tunables::{
    hist_bonus_base, hist_bonus_max, hist_bonus_mult, hist_malus_base, hist_malus_max, hist_malus_mult,
};

/// Entry within a history table.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct HistEntry(i16);

impl HistEntry {
    /// History gravity.
    /// <https://www.chessprogramming.org/History_Heuristic>
    #[allow(clippy::cast_possible_truncation)]
    pub const fn gravity<const MAX: i32>(&mut self, bonus: i16) {
        // Do calculations as i32
        let x = self.0 as i32;
        let b = bonus as i32;
        self.0 += (b - x * b.abs() / MAX) as i16;
    }
}

/// Get the bonus and malus for history at a given depth.
pub fn hist_delta(depth: Depth) -> (i16, i16) {
    let bonus = hist_bonus_max().min(hist_bonus_mult() * i32::from(depth) - hist_bonus_base());
    let malus = hist_malus_max().min(hist_malus_mult() * i32::from(depth) - hist_malus_base());
    // SAFETY: (bonus|malus)_max are both in i16 range.
    (bonus as i16, malus as i16)
}
