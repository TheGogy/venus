use crate::tunables::params::tunables::tt_replace_d_min;

use super::entry::{Bound, CompressedEntry, TTEntry};
use chess::types::{eval::Eval, moves::Move, zobrist::Hash};

/// Transposition table.
pub struct TT {
    entries: Vec<CompressedEntry>,
    age: u8,
}

impl Default for TT {
    fn default() -> Self {
        let mut tt = Self { entries: Vec::new(), age: 0 };

        tt.resize(Self::DEFAULT_SIZE);
        tt
    }
}

impl TT {
    /// Default size in MB for the table.
    pub const DEFAULT_SIZE: usize = 16;

    /// Resize the table to the given size (mb).
    pub fn resize(&mut self, new_size_mb: usize) {
        let entries_count = (new_size_mb << 20) / size_of::<CompressedEntry>();
        self.entries.resize_with(entries_count, CompressedEntry::default);
    }

    /// Get the index for a given hash.
    #[inline]
    const fn index(&self, hash: Hash) -> usize {
        let key = hash.key as u128;
        let len = self.entries.len() as u128;
        ((key * len) >> 64) as usize
    }

    /// Probe the table with some hash.
    pub fn probe(&self, hash: Hash) -> Option<TTEntry> {
        let index = self.index(hash);
        unsafe { self.entries.get_unchecked(index).read(hash) }
    }

    /// Prefetch an entry into the cache.
    #[inline]
    pub fn prefetch(&self, hash: Hash) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            let index = self.index(hash);
            let entry = self.entries.get_unchecked(index);
            _mm_prefetch((entry as *const CompressedEntry).cast::<i8>(), _MM_HINT_T0);
        }
    }

    /// Increment the table age.
    #[inline]
    pub const fn increment_age(&mut self) {
        self.age = (self.age + 1) & 0x7F;
    }

    /// Calculate table utilization (0 - 1000)
    #[inline]
    pub fn hashfull(&self) -> usize {
        let sample_size = 1000.min(self.entries.len());
        self.entries[..sample_size].iter().filter(|e| e.is_occupied()).count()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert(&self, hash: Hash, bound: Bound, mov: Move, eval: Eval, value: Eval, depth: usize, ply: usize, pv: bool) {
        let index = self.index(hash);
        let slot = unsafe { self.entries.get_unchecked(index) };
        let old = slot.read_unchecked();

        let same_position = hash.key == old.key;

        let should_replace = self.age != old.age ||   // Always replace older entries
            !same_position ||                               // Always replace different positions
            bound == Bound::Exact ||                        // Always replace with exact scores
            depth + tt_replace_d_min() + 2 * usize::from(pv) > old.depth as usize; // Replace if deeper

        if should_replace {
            let new_move = if mov.is_null() && same_position { old.mov } else { mov };
            slot.write(TTEntry::new(hash.key, self.age, depth as u8, bound, new_move, eval, value.to_corrected(ply)));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tt::{
        entry::{Bound, CompressedEntry},
        table::TT,
    };
    use chess::types::{eval::Eval, moves::Move, zobrist::Hash};

    #[test]
    fn test_tt_init() {
        let mut tt = TT::default();
        tt.resize(1);

        assert_eq!(16, size_of::<CompressedEntry>());
        assert_eq!(65536, tt.entries.len());
    }

    #[test]
    fn test_tt_insert() {
        let tt = TT::default();
        let mut z = Hash::default();

        tt.insert(z, Bound::Exact, Move(1), Eval(100), Eval(100), 1, 0, false);
        tt.insert(z, Bound::Exact, Move(1), Eval(100), Eval(100), 12, 0, false);
        tt.insert(z, Bound::Upper, Move(1), Eval(100), Eval(100), 1, 0, false);

        let target1 = tt.probe(z).unwrap();
        z.key = 1;
        let target2 = tt.probe(z);

        assert_eq!(12, target1.depth());
        assert!(target2.is_none());
    }
}
