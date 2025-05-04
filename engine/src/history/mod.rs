pub mod noisyhist;
pub mod quiethist;

pub mod movebuffer;

pub mod hist;
pub use hist::*;

/// Entry within a history table.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct HistEntry(i16);

impl HistEntry {
    /// History gravity
    /// https://www.chessprogramming.org/History_Heuristic
    pub const fn gravity<const MAX: i32>(&mut self, bonus: i16) {
        // Do calculations as i32
        let x = self.0 as i32;
        let b = bonus as i32;
        self.0 = (x + b - (x * b.abs()) / MAX) as i16
    }
}
