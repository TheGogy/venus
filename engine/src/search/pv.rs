use chess::{
    MAX_PLY,
    types::{castling::CastlingMask, moves::Move},
};

use std::fmt::Write;

/// PVLine.
/// This allows us to keep track of the current PV.
#[derive(Clone, Debug)]
pub struct PVLine {
    pub moves: [Move; MAX_PLY],
    length: usize,
}

impl Default for PVLine {
    fn default() -> Self {
        Self { moves: [Move::NONE; MAX_PLY], length: 0 }
    }
}

impl PVLine {
    /// Update the PV line with a child line.
    pub fn update(&mut self, m: Move, child: &Self) {
        debug_assert!(child.length < MAX_PLY);

        self.length = child.length + 1;
        self.moves[0] = m;
        self.moves[1..=child.length].copy_from_slice(&child.moves[..child.length]);
    }

    /// Clear the PV line.
    pub const fn clear(&mut self) {
        self.length = 0;
    }

    /// Print out the PV according to UCI format.
    pub fn to_uci(&self, cm: &CastlingMask) -> String {
        let mut s = String::from("pv");

        for m in &self.moves[..self.length] {
            let _ = write!(s, " {}", m.to_uci(cm));
        }

        s
    }
}
