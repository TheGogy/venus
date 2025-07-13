use chess::Depth;

use crate::tunables::params::tunables::*;

pub mod conthist;
pub mod corrhist;
pub mod movebuffer;
pub mod noisyhist;
pub mod quiethist;

/// Entry within a history table.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct HistEntry(i16);

impl HistEntry {
    /// History gravity.
    /// https://www.chessprogramming.org/History_Heuristic
    pub const fn gravity<const MAX: i32>(&mut self, bonus: i16) {
        // Do calculations as i32
        let x = self.0 as i32;
        let b = bonus as i32;
        self.0 += (b - x * b.abs() / MAX) as i16
    }
}

/// Get the bonus and malus for history at a given depth.
pub fn hist_delta(depth: Depth) -> (i16, i16) {
    let bonus = hist_bonus_max().min(hist_bonus_mult() * depth - hist_bonus_base());
    let malus = hist_malus_max().min(hist_malus_mult() * depth - hist_malus_base());
    (bonus, malus)
}
