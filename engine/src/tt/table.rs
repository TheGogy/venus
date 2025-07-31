use chess::types::{Depth, eval::Eval, moves::Move, zobrist::Hash};

use super::{
    bits::{Bound, MAX_AGE},
    entry::{TTBucket, TTHit, TTRef},
};

/// Transposition table.
pub struct TT {
    buckets: Vec<TTBucket>,
    age: u8,
}

impl Default for TT {
    fn default() -> Self {
        let mut tt = Self { buckets: Vec::new(), age: 0 };
        tt.resize(Self::DEFAULT_SIZE);
        tt
    }
}

impl TT {
    /// Default size in MB for the table.
    pub const DEFAULT_SIZE: usize = 16;

    /// Resize the table to the given size (mb).
    pub fn resize(&mut self, new_size_mb: usize) {
        let entries_count = (new_size_mb << 20) / size_of::<TTBucket>();
        self.buckets.resize_with(entries_count, TTBucket::default);
    }

    /// Get the index for a given hash.
    const fn idx(&self, hash: Hash) -> usize {
        let key = hash.key as u128;
        let len = self.buckets.len() as u128;
        ((key * len) >> 64) as usize
    }

    /// Probe the table with some hash.
    pub fn probe(&self, hash: Hash) -> (TTHit, TTRef) {
        let index = self.idx(hash);
        unsafe { self.buckets.get_unchecked(index).probe(index, hash.key, self.age) }
    }

    /// Prefetch an entry into the cache.
    #[allow(unused_variables)]
    pub fn prefetch(&self, hash: Hash) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            let index = self.idx(hash);
            let entry = self.buckets.get_unchecked(index);
            _mm_prefetch::<_MM_HINT_T0>((entry as *const TTBucket).cast());
        }
    }

    /// Increment the table age.
    pub const fn increment_age(&mut self) {
        self.age = (self.age + 1) & MAX_AGE;
    }

    /// Calculate table utilization (0 - 1000).
    pub fn hashfull(&self) -> usize {
        let sample_size = 1000.min(self.buckets.len());
        self.buckets[0..sample_size].iter().map(|b| b.count_valid(self.age)).sum()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &self,
        tt_ref: TTRef,
        hash: Hash,
        bound: Bound,
        mov: Move,
        eval: Eval,
        value: Eval,
        depth: Depth,
        ply: usize,
        is_pv: bool,
    ) {
        let adj_value = value.to_corrected(ply).0 as i16;
        let adj_eval = eval.0 as i16;
        let bucket = unsafe { self.buckets.get_unchecked(tt_ref.bucket_index) };
        let key = hash.key;

        bucket.entries[tt_ref.entry_index].update(key, adj_eval, self.age, is_pv, bound, depth, mov, adj_value);
    }

    /// Clear the transposition table.
    pub fn clear(&mut self) {
        self.age = 0;
        self.buckets.iter_mut().for_each(|e| *e = TTBucket::default());
    }
}
