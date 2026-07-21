use std::sync::atomic::{AtomicU64, Ordering};

use chess::types::{Depth, eval::Eval, moves::Move, zobrist::Hash};

/// TT depth offsets.
/// Depth of qsearch entries. Must be <= -1, as we add 1 for qsearch entries in check.
pub const TT_DEPTH_QS: Depth = -1;

/// Depth of unsearched entries. Must be < QS entries.
pub const TT_DEPTH_UNSEARCHED: Depth = -2;

/// Offset to make all stored depths positive and > 0.
pub const TT_DEPTH_OFFSET: Depth = 3;

/// Number of entries to store in each bucket.
pub const TT_BUCKET_ENTRIES: usize = 3;

/// Size of each TT bucket.
pub const TT_BUCKET_SIZE: usize = std::mem::size_of::<TTBucket>();
const TT_BUCKET_WORDS: usize = TT_BUCKET_SIZE / std::mem::size_of::<u64>();

pub const TT_AGE_CYCLE: u8 = 1 << 5;
pub const TT_AGE_MASK: u8 = TT_AGE_CYCLE - 1;

/// Penalty applied to older entries when choosing a replacement victim.
pub const TT_AGE_MUL: i32 = 8;

/// Get the partial key stored in each entry.
pub const fn get_low_16(hash: Hash) -> u16 {
    (hash.key & 0xFFFF) as u16
}

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

/// Packed entry metadata.
///
/// Bits:
/// - 0..=1: bound
/// - 2: PV flag
/// - 3..=7: generation age
#[derive(Clone, Copy, Debug, Default)]
pub struct TTMetadata(u8);

impl TTMetadata {
    pub const fn new(age: u8, pv: bool, bound: Bound) -> Self {
        Self((bound as u8) | ((pv as u8) << 2) | (age << 3))
    }

    pub const fn age(self) -> u8 {
        self.0 >> 3
    }

    pub const fn pv(self) -> bool {
        (self.0 & 0b100) != 0
    }

    pub const fn bound(self) -> Bound {
        unsafe { std::mem::transmute(self.0 & 0b11) }
    }
}

/// Transposition table entry.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TTEntry {
    pub key: u16,             // 2 bytes
    pub mov: Move,            // 2 bytes
    pub eval: i16,            // 2 bytes
    pub value: i16,           // 2 bytes
    pub depth: u8,            // 1 byte
    pub metadata: TTMetadata, // 1 byte
}

const _: () = assert!(std::mem::size_of::<TTEntry>() == 10);

impl TTEntry {
    pub const fn mov(&self) -> Move {
        self.mov
    }

    pub const fn eval(&self) -> Eval {
        Eval(self.eval as i32)
    }

    pub const fn value(&self, ply: usize) -> Eval {
        Eval(self.value as i32).from_tb_score(ply)
    }

    pub const fn depth(&self) -> Depth {
        (self.depth as Depth) - TT_DEPTH_OFFSET
    }

    pub const fn bound(&self) -> Bound {
        self.metadata.bound()
    }

    pub const fn pv(&self) -> bool {
        self.metadata.pv()
    }

    pub const fn is_occupied(&self) -> bool {
        self.depth > 0
    }

    pub const fn key_matches(&self, hash: Hash) -> bool {
        self.key == get_low_16(hash)
    }

    /// Check whether this entry is occupied and matches the stored partial key.
    pub const fn matches(&self, hash: Hash) -> bool {
        self.is_occupied() && self.key_matches(hash)
    }

    /// Age distance from the current table generation, modulo the age cycle.
    pub const fn relative_age(&self, tt_age: u8) -> i32 {
        ((TT_AGE_CYCLE + tt_age - self.metadata.age()) & TT_AGE_MASK) as i32
    }

    /// Replacement priority. Lower values are replaced first.
    pub const fn relative_quality(&self, tt_age: u8) -> i32 {
        self.depth as i32 - TT_AGE_MUL * self.relative_age(tt_age)
    }
}

/// One cache-line-sized bucket of TT entries.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(32))]
pub struct TTBucket {
    pub entries: [TTEntry; TT_BUCKET_ENTRIES], // 30 bytes
    hash: u16,                                 // 2 bytes
}

impl TTBucket {
    const fn checksum(&self) -> u16 {
        self.entries[0].key ^ self.entries[1].key ^ self.entries[2].key
    }

    pub const fn update_checksum(&mut self) {
        self.hash = self.checksum();
    }

    pub const fn checksum_matches(&self) -> bool {
        self.hash == self.checksum()
    }
}

/// Atomic bucket storage.
#[derive(Debug, Default)]
#[repr(C, align(32))]
pub struct AtomicTTBucket {
    data: [AtomicU64; TT_BUCKET_WORDS], // 32 bytes
}

const _: () = assert!(std::mem::size_of::<TTBucket>() == 32);
const _: () = assert!(std::mem::size_of::<AtomicTTBucket>() == 32);

impl AtomicTTBucket {
    pub fn load(&self) -> TTBucket {
        let raw: [u64; TT_BUCKET_WORDS] = std::array::from_fn(|i| self.data[i].load(Ordering::Relaxed));
        unsafe { std::mem::transmute(raw) }
    }

    pub fn store(&self, bucket: TTBucket) {
        let raw: [u64; TT_BUCKET_WORDS] = unsafe { std::mem::transmute(bucket) };
        for (slot, word) in self.data.iter().zip(raw) {
            slot.store(word, Ordering::Relaxed);
        }
    }
}
