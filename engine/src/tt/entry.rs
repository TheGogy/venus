use std::{
    cmp,
    sync::atomic::{AtomicU64, Ordering},
};

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
    Upper,
    Lower,
    Exact,
    None,
}

/// Entry in the transposition table
/// TODO: Add is_pv?
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
pub struct TTEntry {
    pub key: u64,     // 64 bits - Position hash.
    pub age: u8,      //  7 bits - Search generation.
    pub depth: u8,    //  7 bits - Search depth.
    pub bound: Bound, //  2 bits - Type of bound.
    pub mov: Move,    // 16 bits - Best move found.
    pub eval: i16,    // 16 bits - Static evaluation.
    pub value: i16,   // 16 bits - Search score.
}

/// A compressed transposition table entry stored in the table.
#[derive(Debug, Default)]
pub struct CompressedEntry {
    key: AtomicU64,
    data: AtomicU64,
}

impl TTEntry {
    /// Make a new TT entry.
    pub const fn new(key: u64, age: u8, depth: u8, bound: Bound, mov: Move, eval: Eval, value: Eval) -> Self {
        Self { key, age, depth, bound, mov, eval: eval.0 as i16, value: value.0 as i16 }
    }

    /// Compress this TT entry.
    pub const fn compress(self) -> (u64, u64) {
        let data = (self.age as u64)
            | bits::pack_depth(self.depth)
            | bits::pack_bound(self.bound)
            | bits::pack_move(self.mov)
            | bits::pack_eval(self.eval)
            | bits::pack_value(self.value);

        (self.key ^ data, data)
    }

    /// Get a TT entry from compressed format.
    pub const fn from_compressed(key: u64, data: u64) -> Self {
        Self {
            key: key ^ data, // Recover original key
            age: bits::unpack_age(data),
            depth: bits::unpack_depth(data),
            bound: bits::unpack_bound(data),
            mov: bits::unpack_move(data),
            eval: bits::unpack_eval(data),
            value: bits::unpack_value(data),
        }
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

    /// Get the tightest bound with the current eval.
    pub fn get_tightest(self, eval: Eval, ply: usize) -> Eval {
        match self.bound() {
            Bound::Exact => self.value(ply),
            Bound::Upper => cmp::min(eval, self.value(ply)),
            Bound::Lower => cmp::max(eval, self.value(ply)),
            Bound::None => eval,
        }
    }
}

impl CompressedEntry {
    /// Read the entry, with hash verification.
    pub fn read(&self, hash: Hash) -> Option<TTEntry> {
        let key = self.key.load(Ordering::Relaxed);
        let data = self.data.load(Ordering::Relaxed);

        // Verify the entry matches the expected hash
        if key ^ hash.key == data { Some(TTEntry::from_compressed(key, data)) } else { None }
    }

    /// Read without verification - only use when known valid.
    pub fn read_unchecked(&self) -> TTEntry {
        let key = self.key.load(Ordering::Relaxed);
        let data = self.data.load(Ordering::Relaxed);
        TTEntry::from_compressed(key, data)
    }

    /// Write an entry to the table.
    pub fn write(&self, entry: TTEntry) {
        let (key, data) = entry.compress();
        self.key.store(key, Ordering::Relaxed);
        self.data.store(data, Ordering::Relaxed);
    }

    /// Check if this entry is occupied.
    pub fn is_occupied(&self) -> bool {
        self.key.load(Ordering::Relaxed) != 0
    }
}
