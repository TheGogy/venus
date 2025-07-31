use std::cell::UnsafeCell;

use chess::types::{Depth, eval::Eval, moves::Move};

use super::bits::{AgePVBound, Bound};

/// TT hit.
/// We return this when we get a hit in the TT.
#[derive(Debug, Default)]
pub struct TTHit {
    pub eval: Eval,
    pub depth: Depth,
    pub was_pv: bool,
    pub bound: Bound,
    pub mov: Move,
    pub value: Eval,
}

impl From<TTEntry> for TTHit {
    fn from(entry: TTEntry) -> Self {
        Self {
            eval: Eval(entry.eval as i32),
            depth: (entry.depth as Depth) + TT_DEPTH_OFFSET,
            was_pv: entry.data.is_pv(),
            bound: entry.data.bound(),
            mov: entry.mov,
            value: Eval(entry.value as i32),
        }
    }
}

// TT depth offsets.
pub const TT_DEPTH_QS: Depth = -1;
// pub const TT_DEPTH_UNSEARCHED: Depth = -2; // TODO
pub const TT_DEPTH_OFFSET: Depth = -3;

/// Entry in the transposition table.
#[derive(Debug, Default, Copy, Clone)]
pub struct TTEntry {
    pub key: u16,         // 16 bits - First 16 bits of position hash.
    pub eval: i16,        // 16 bits - Static eval.
    pub data: AgePVBound, // 8  bits - The age, PV and bound type.
    pub depth: u8,        // 8  bits - Search depth.
    pub mov: Move,        // 16 bits - The best move.
    pub value: i16,       // 16 bits - The search score.
}

impl TTEntry {
    pub const fn same_key(&self, key: u64) -> bool {
        key as u16 == self.key
    }

    pub const fn quality(&self, table_age: u8) -> u8 {
        self.depth - (self.data.relative_age(table_age) << 3)
    }
}

#[derive(Debug, Default)]
pub struct TTSlot {
    entry: UnsafeCell<TTEntry>,
}

unsafe impl Sync for TTSlot {}

impl TTSlot {
    #[allow(clippy::too_many_arguments)]
    pub fn update(&self, key: u64, eval: i16, table_age: u8, is_pv: bool, bound: Bound, depth: Depth, mov: Move, value: i16) {
        let e = unsafe { &mut *self.entry.get() };

        if !mov.is_none() || !e.same_key(key) {
            e.mov = mov
        }

        assert!(depth > TT_DEPTH_OFFSET);
        let adj_depth = (depth - TT_DEPTH_OFFSET) as u8;

        // Overwrite less valuable entries.
        if bound == Bound::Exact || !e.same_key(key) || e.data.relative_age(table_age) > 0 || adj_depth + 4 + 2 * (is_pv as u8) > e.depth {
            e.key = key as u16;
            e.eval = eval;
            e.data = AgePVBound::from(bound, is_pv, table_age);
            e.depth = adj_depth;
            e.mov = mov;
            e.value = value;
        }
    }

    #[inline(always)]
    pub fn read(&self) -> TTEntry {
        unsafe { *self.entry.get() }
    }
}

const BUCKET_SIZE: usize = 3;

// Reference to the TT entry to overwrite.
#[derive(Debug, Default, Clone, Copy)]
pub struct TTRef {
    pub hit: bool,
    pub bucket_index: usize,
    pub entry_index: usize,
}

// Bucket of TT entries.
#[derive(Debug, Default)]
pub struct TTBucket {
    pub entries: [TTSlot; BUCKET_SIZE],

    #[allow(unused)]
    pad: u16,
}

impl TTBucket {
    /// Probe this bucket for an entry.
    pub fn probe(&self, bucket_index: usize, key: u64, table_age: u8) -> (TTHit, TTRef) {
        // Find an entry we can use if possible.
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.read().same_key(key) {
                let r = TTRef { hit: entry.read().data.is_valid(), bucket_index, entry_index: i };
                return (TTHit::from(entry.read()), r);
            }
        }

        // Otherwise find the entry we're going to replace.
        let mut worst_idx = 0;
        for (i, entry) in self.entries[1..].iter().enumerate() {
            if entry.read().quality(table_age) < self.entries[worst_idx].read().quality(table_age) {
                worst_idx = i
            }
        }

        let r = TTRef { hit: false, bucket_index, entry_index: worst_idx };
        (TTHit::default(), r)
    }

    /// Count the number of entries being used in this bucket.
    pub fn count_valid(&self, table_age: u8) -> usize {
        self.entries.iter().filter(|e| e.read().data.is_valid() && e.read().data.age() == table_age).count()
    }
}
