use core::fmt;

use chess::{MAX_DEPTH, types::moves::Move};
use const_default::ConstDefault;

/// PVLine.
/// This allows us to keep track of the current PV.
#[derive(Clone, Debug)]
pub struct PVLine {
    pub moves: [Move; MAX_DEPTH],
    length: usize,
}

impl ConstDefault for PVLine {
    const DEFAULT: Self = Self { moves: [Move::NULL; MAX_DEPTH], length: 0 };
}

impl std::fmt::Display for PVLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::from("pv");

        for m in &self.moves[0..self.length] {
            s.push_str(&format!(" {m}"));
        }

        write!(f, "{s}")
    }
}

impl PVLine {
    /// Update the PV line with a child line.
    #[inline]
    pub fn update(&mut self, m: Move, child: &Self) {
        self.length = child.length + 1;
        self.moves[0] = m;
        self.moves[1..=child.length].copy_from_slice(&child.moves[..child.length]);
    }

    /// Clear the PV line.
    pub const fn clear(&mut self) {
        self.length = 0;
    }
}
