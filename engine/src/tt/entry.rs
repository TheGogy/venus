use std::sync::atomic::{AtomicU64, Ordering};

use chess::types::{eval::Eval, moves::Move, zobrist::Hash};

use super::bits;

/// TT Bound.
/// Upper: search at this position fails high.
/// Lower: search at this position fails low.
/// Exact: exact value of this node.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
#[repr(u8)]
pub enum Bound {
    #[default]
    None,
    Upper,
    Lower,
    Exact,
}

impl Bound {
    /// Whether this bound constitutes the given bound.
    pub const fn has(self, other: Bound) -> bool {
        (self as u8) & (other as u8) != 0
    }

    /// Whether or not we can use this bound.
    pub fn is_usable(self, eval: Eval, other: Eval) -> bool {
        self.has(if eval >= other { Self::Lower } else { Self::Upper })
    }
}

/// Entry in the transposition table
/// TODO: Add is_pv?
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
pub struct TTEntry {
    pub key: u64,     // 64 bits - Position hash.
    pub pv: bool,     //  1 bit  - Whether this position was on the pv.
    pub age: u8,      //  6 bits - Search generation.
    pub depth: u8,    //  7 bits - Search depth.
    pub bound: Bound, //  2 bits - Type of bound.
    pub mov: Move,    // 16 bits - Best move found.
    pub eval: i16,    // 16 bits - Static evaluation.
    pub value: i16,   // 16 bits - Search score.
}

/// A packed transposition table entry stored in the table.
#[derive(Debug, Default)]
pub struct PackedTTEntry {
    key: AtomicU64,
    data: AtomicU64,
}

impl TTEntry {
    /// Make a new TT entry.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(key: u64, pv: bool, age: u8, depth: u8, bound: Bound, mov: Move, eval: Eval, value: Eval) -> Self {
        Self { key, pv, age, depth, bound, mov, eval: eval.0 as i16, value: value.0 as i16 }
    }

    /// Pack this TT entry.
    pub const fn pack(self) -> (u64, u64) {
        let data = bits::pack_pv(self.pv)
            | bits::pack_age(self.age)
            | bits::pack_depth(self.depth)
            | bits::pack_bound(self.bound)
            | bits::pack_move(self.mov)
            | bits::pack_eval(self.eval)
            | bits::pack_value(self.value);

        (self.key ^ data, data)
    }

    /// Get a TT entry from packed format.
    pub const fn unpack(key: u64, data: u64) -> Self {
        Self {
            key: key ^ data, // Recover original key
            pv: bits::unpack_pv(data),
            age: bits::unpack_age(data),
            depth: bits::unpack_depth(data),
            bound: bits::unpack_bound(data),
            mov: bits::unpack_move(data),
            eval: bits::unpack_eval(data),
            value: bits::unpack_value(data),
        }
    }

    /// Get whether this was on the pv.
    pub const fn pv(self) -> bool {
        self.pv
    }

    /// Get the depth.
    pub const fn depth(self) -> i16 {
        self.depth as i16
    }

    /// Get the bound.
    pub const fn bound(self) -> Bound {
        self.bound
    }

    /// Get the move.
    pub const fn mov(self) -> Move {
        self.mov
    }

    /// Get static evaluation.
    pub const fn eval(self) -> Eval {
        Eval(self.eval as i32)
    }

    /// Get the search score.
    pub const fn value(self, ply: usize) -> Eval {
        Eval(self.value as i32).from_corrected(ply)
    }
}

impl PackedTTEntry {
    /// Read the entry, with hash verification.
    pub fn read(&self, hash: Hash) -> Option<TTEntry> {
        let key = self.key.load(Ordering::Relaxed);
        let data = self.data.load(Ordering::Relaxed);

        // Verify the entry matches the expected hash
        if key ^ hash.key == data { Some(TTEntry::unpack(key, data)) } else { None }
    }

    /// Read without verification - only use when known valid.
    pub fn read_unchecked(&self) -> TTEntry {
        let key = self.key.load(Ordering::Relaxed);
        let data = self.data.load(Ordering::Relaxed);
        TTEntry::unpack(key, data)
    }

    /// Write an entry to the table.
    pub fn write(&self, entry: TTEntry) {
        let (key, data) = entry.pack();

        self.key.store(key, Ordering::Relaxed);
        self.data.store(data, Ordering::Relaxed);
    }

    /// Check if this entry is occupied.
    pub fn is_occupied(&self) -> bool {
        self.key.load(Ordering::Relaxed) != 0
    }
}

#[cfg(test)]
mod tests {
    use chess::types::{eval::Eval, moves::Move};

    use super::{Bound, TTEntry};

    #[test]
    fn test_entry_pack_unpack() {
        let entries = vec![TTEntry::new(11798647534405270886, false, 0, 255, Bound::Upper, Move::NONE, Eval(18), Eval(18))];

        for e in entries {
            let (key, data) = e.pack();
            assert_eq!(TTEntry::unpack(key, data), e);
        }
    }
}
