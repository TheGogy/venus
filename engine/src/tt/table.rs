use chess::types::{Depth, eval::Eval, moves::Move, zobrist::Hash};

use crate::{
    tt::entry::{AtomicTTBucket, Bound, TT_AGE_MASK, TT_BUCKET_SIZE, TT_DEPTH_OFFSET, TTBucket, TTEntry, TTMetadata, get_low_16},
    tunables::params::tunables::tt_replace_d_min,
};

/// Transposition table.
pub struct TT {
    buckets: Vec<AtomicTTBucket>,
    age: u8,
}

const MEGABYTE: usize = 1024 * 1024;

impl Default for TT {
    fn default() -> Self {
        Self::with_size(Self::DEFAULT_SIZE_MB)
    }
}

impl TT {
    /// Default table size in megabytes.
    pub const DEFAULT_SIZE_MB: usize = 16;

    /// Create a table with approximately `size_mb` megabytes of storage.
    pub fn with_size(size_mb: usize) -> Self {
        let mut tt = Self { buckets: Vec::new(), age: 0 };
        tt.resize(size_mb);
        tt
    }

    /// Resize the table and restart the generation counter.
    pub fn resize(&mut self, size_mb: usize) {
        let n_buckets = size_mb * MEGABYTE / TT_BUCKET_SIZE;
        self.buckets.resize_with(n_buckets, AtomicTTBucket::default);
        self.age = 0;
    }

    /// Clear all entries and reset the generation counter.
    pub fn clear(&mut self) {
        self.age = 0;
        self.buckets.iter_mut().for_each(|bucket| *bucket = AtomicTTBucket::default());
    }

    /// Advance to the next search generation.
    pub fn increment_age(&mut self) {
        self.age = (self.age + 1) & TT_AGE_MASK;
    }

    /// Estimate table occupancy in permille.
    pub fn hashfull(&self) -> usize {
        self.buckets.iter().flat_map(|bucket| bucket.load().entries).take(1000).filter(|entry| entry.is_occupied()).count()
    }

    /// Map a full hash to a bucket index using high-multiply reduction.
    const fn idx(&self, hash: Hash) -> usize {
        let key = hash.key as u128;
        let len = self.buckets.len() as u128;
        ((key * len) >> 64) as usize
    }

    /// Prefetch the bucket corresponding to the given hash.
    #[allow(unused_variables)]
    pub fn prefetch(&self, hash: Hash) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            let index = self.idx(hash);
            let bucket = self.buckets.get_unchecked(index);
            _mm_prefetch::<_MM_HINT_T0>(std::ptr::from_ref(bucket).cast());
        }
    }

    /// Probe the TT for an entry matching the given hash.
    pub fn probe(&self, hash: Hash) -> Option<TTEntry> {
        let bucket: TTBucket = self.buckets[self.idx(hash)].load();

        if !bucket.checksum_matches() {
            return None;
        }

        bucket.entries.iter().find(|&entry| entry.matches(hash)).copied()
    }

    /// Insert or update an entry.
    #[allow(clippy::too_many_arguments)]
    pub fn insert(&self, hash: Hash, bound: Bound, mov: Move, eval: Eval, value: Eval, depth: Depth, ply: usize, pv: bool) {
        let slot = &self.buckets[self.idx(hash)];
        let mut bucket: TTBucket = slot.load();

        let mut index = 0;
        let mut worst = i32::MAX;

        for (i, entry) in bucket.entries.iter().enumerate() {
            if entry.matches(hash) {
                index = i;
                break;
            }

            let quality = entry.relative_quality(self.age);
            if quality < worst {
                worst = quality;
                index = i;
            }
        }

        let entry = &mut bucket.entries[index];

        // Force move update on new position, or if we have a move now.
        // Otherwise, preserve old move.
        if !mov.is_none() || !entry.matches(hash) {
            entry.mov = mov;
        }

        if bound == Bound::Exact                   // Replace on exact scores.
            || !entry.matches(hash)                // Replace different positions.
            || entry.metadata.age() != self.age    // Replace older entries.
            || depth + tt_replace_d_min() + 2 * Depth::from(pv) > entry.depth()
        {
            entry.key = get_low_16(hash);
            entry.eval = eval.0.try_into().expect("Eval exceeds i16");
            entry.value = value.to_tb_score(ply).0.try_into().expect("Value exceeds i16");
            entry.depth = (depth + TT_DEPTH_OFFSET) as u8;
            entry.metadata = TTMetadata::new(self.age, pv, bound);
        }

        bucket.update_checksum();
        slot.store(bucket);
    }
}

#[cfg(test)]
mod tests {
    use chess::types::{eval::Eval, moves::Move, zobrist::Hash};

    use crate::tt::{entry::Bound, table::TT};

    #[test]
    fn test_insert_roundtrip() {
        let tt = TT::with_size(1);
        let h = Hash { key: 0x1234_5678_9ABC_DEF0, ..Hash::default() };

        tt.insert(h, Bound::Lower, Move(42), Eval(123), Eval(456), 8, 3, true);

        let entry = tt.probe(h).unwrap();
        assert_eq!(Move(42), entry.mov());
        assert_eq!(Eval(123), entry.eval());
        assert_eq!(Eval(456), entry.value(3));
        assert_eq!(8, entry.depth());
        assert_eq!(Bound::Lower, entry.bound());
        assert!(entry.pv());
    }
}
