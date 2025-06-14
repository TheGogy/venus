use chess::{
    MAX_DEPTH,
    types::{castling::CastlingMask, moves::Move},
};

/// PVLine.
/// This allows us to keep track of the current PV.
#[derive(Clone, Debug)]
pub struct PVLine {
    pub moves: [Move; MAX_DEPTH],
    length: usize,
}

impl Default for PVLine {
    fn default() -> Self {
        Self { moves: [Move::NULL; MAX_DEPTH], length: 0 }
    }
}

impl PVLine {
    /// Update the PV line with a child line.
    pub fn update(&mut self, m: Move, child: &Self) {
        debug_assert!(child.length < MAX_DEPTH);

        self.length = child.length + 1;
        self.moves[0] = m;

        // SAFETY: We've asserted that child.length < MAX_DEPTH,
        // so child.length is a valid index, and the slice is valid.
        unsafe {
            let src = child.moves.get_unchecked(..child.length);
            let dst = self.moves.get_unchecked_mut(1..=child.length);
            dst.copy_from_slice(src);
        }
    }

    /// Clear the PV line.
    pub const fn clear(&mut self) {
        self.length = 0;
    }

    /// Print out the PV according to UCI format.
    pub fn to_uci(&self, cm: &CastlingMask) -> String {
        let mut s = String::from("pv");

        for m in &self.moves[0..self.length] {
            s.push_str(&format!(" {}", m.to_uci(cm)));
        }

        s
    }
}
