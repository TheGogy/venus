use std::sync::atomic::{AtomicU64, Ordering};

use chess::types::{Depth, eval::Eval, moves::Move, zobrist::Hash};

use super::bits;

/// TT Bound.
/// Upper: search at this position fails high.
/// Lower: search at this position fails low.
/// Exact: exact value of this node.
#[rustfmt::skip]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
#[repr(u8)]
pub enum Bound {
    #[default]
    None  = 0b00,
    Upper = 0b01,
    Lower = 0b10,
    Exact = 0b11,
}

impl Bound {
    /// Whether this bound contains the other bound.
    pub const fn has(self, other: Bound) -> bool {
        self as u8 & other as u8 != 0
    }

    /// Whether the given eval is usable given the operand.
    pub const fn is_usable(self, eval: Eval, operand: Eval) -> bool {
        self.has(if eval.0 >= operand.0 { Self::Lower } else { Self::Upper })
    }
}

/// Entry in the transposition table
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
pub struct TTEntry {
    pub key: u64,     // 64 bits - Position hash.
    pub pv: bool,     //  1 bit  - Whether this node was pv.
    pub age: u8,      //  6 bits - Search generation.
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

pub const TT_DEPTH_QS: Depth = -1;
const TT_DEPTH_OFFSET: Depth = 2;

impl TTEntry {
    /// Make a new TT entry.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(key: u64, pv: bool, age: u8, d: Depth, bound: Bound, mov: Move, eval: Eval, value: Eval) -> Self {
        let depth = (d + TT_DEPTH_OFFSET) as u8;
        Self { key, pv, age, depth, bound, mov, eval: eval.0 as i16, value: value.0 as i16 }
    }

    /// Compress this TT entry.
    pub const fn compress(self) -> (u64, u64) {
        let data = bits::pack_pv(self.pv)
            | bits::pack_age(self.age)
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
            pv: bits::unpack_pv(data),
            age: bits::unpack_age(data),
            depth: bits::unpack_depth(data),
            bound: bits::unpack_bound(data),
            mov: bits::unpack_move(data),
            eval: bits::unpack_eval(data),
            value: bits::unpack_value(data),
        }
    }

    /// Get whether this was a pv node.
    pub const fn pv(self) -> bool {
        self.pv
    }

    /// Get the depth.
    pub const fn depth(self) -> Depth {
        (self.depth as Depth) - TT_DEPTH_OFFSET
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
