//! Calculate the FTPERM_INDICES to minimize the number of blocks activated during inference.

use std::fs;
use std::io::{self, Read};

use nnue::arch::PAIRWISE_LEN;
use nnue::inference::sparse::{ACTS_DUMP_FILE, ACTS_MAGIC};
use rand::prelude::*;

const BLOCK_SIZE: usize = 4;
const N_BLOCKS: usize = PAIRWISE_LEN / BLOCK_SIZE;

/// u64s per AVX-512 register.
const WORDS_PER_LINE: usize = 512 / 64;

/// Total steps to anneal for.
const STEPS: usize = 2_000_000;

/// Probability a *typical* uphill move will accepted at the start of the schedule.
const START_ACCEPT: f64 = 0.5;

/// End temperature as a fraction of the start.
const END_TEMP_RATIO: f64 = 1.0 / 400.0;

/// Swaps sampled to estimate the cost scale.
const CALIBRATION_SAMPLES: usize = 512;

struct Activations {
    /// Total words that hold samples.
    words: usize,
    /// Words between the start of consecutive columns.
    stride: usize,
    /// Index of the first word of column 0.
    start_idx: usize,
    /// No. of positions logged.
    positions: usize,
    /// Samples.
    data: Vec<u64>,
}

impl Activations {
    fn load_from_file(path: &str) -> io::Result<Self> {
        let mut file = io::BufReader::new(fs::File::open(path)?);

        let mut magic = [0u8; ACTS_MAGIC.len()];
        file.read_exact(&mut magic)?;
        if &magic != ACTS_MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Incorrect magic for '{path}'!")));
        }

        let mut count = [0u8; size_of::<u64>()];
        file.read_exact(&mut count)?;
        let positions = u64::from_le_bytes(count) as usize;

        let chunks = positions.div_ceil(64);
        let words = 2 * chunks;
        let stride = words.next_multiple_of(WORDS_PER_LINE);

        let mut data = vec![0u64; PAIRWISE_LEN * stride + WORDS_PER_LINE];
        let base = data.as_ptr() as usize / size_of::<u64>();
        let start_idx = base.next_multiple_of(WORDS_PER_LINE) - base;

        let mut chunk = [0u8; 2 * PAIRWISE_LEN * size_of::<u64>()];
        for c in 0..chunks {
            file.read_exact(&mut chunk)?;

            for i in 0..PAIRWISE_LEN {
                let word = |pov: usize| {
                    let at = (pov * PAIRWISE_LEN + i) * size_of::<u64>();
                    u64::from_le_bytes(chunk[at..at + size_of::<u64>()].try_into().unwrap())
                };

                let col = start_idx + i * stride;
                data[col + 2 * c] = word(0);
                data[col + 2 * c + 1] = word(1);
            }
        }

        Ok(Activations { words, stride, start_idx, data, positions })
    }

    /// Samples the cost sums over.
    fn samples(&self) -> usize {
        2 * self.positions
    }

    #[inline]
    fn col(&self, neuron: usize) -> &[u64] {
        &self.data[self.start_idx + neuron * self.stride..][..self.words]
    }

    #[inline]
    fn block(&self, perm: &[usize], block: usize) -> [&[u64]; BLOCK_SIZE] {
        let base = block * BLOCK_SIZE;
        std::array::from_fn(|k| self.col(perm[base + k]))
    }
}

/// Number of positions where at least one neuron fires.
fn block_cost(cols: [&[u64]; BLOCK_SIZE]) -> u64 {
    let n = cols[0].len();
    let (c0, c1, c2, c3) = (&cols[0][..n], &cols[1][..n], &cols[2][..n], &cols[3][..n]);

    let mut count = 0u64;
    for w in 0..n {
        count += (c0[w] | c1[w] | c2[w] | c3[w]).count_ones() as u64;
    }
    count
}
fn block_cost_pair(a: [&[u64]; BLOCK_SIZE], b: [&[u64]; BLOCK_SIZE]) -> (u64, u64) {
    let n = a[0].len();
    let (a0, a1, a2, a3) = (&a[0][..n], &a[1][..n], &a[2][..n], &a[3][..n]);
    let (b0, b1, b2, b3) = (&b[0][..n], &b[1][..n], &b[2][..n], &b[3][..n]);

    let mut count_a = 0u64;
    let mut count_b = 0u64;
    for w in 0..n {
        count_a += (a0[w] | a1[w] | a2[w] | a3[w]).count_ones() as u64;
        count_b += (b0[w] | b1[w] | b2[w] | b3[w]).count_ones() as u64;
    }
    (count_a, count_b)
}

/// Memoises candidate block costs.
struct CostCache {
    entries: Vec<CacheEntry>,
    versions: [u32; N_BLOCKS],
}

#[derive(Clone, Copy)]
struct CacheEntry {
    /// Version of the owning block when `cost` was computed.
    /// 0 = empty.
    version: u32,
    cost: u32,
}

impl CostCache {
    fn new() -> Self {
        CostCache { entries: vec![CacheEntry { version: 0, cost: 0 }; N_BLOCKS * BLOCK_SIZE * PAIRWISE_LEN], versions: [1; N_BLOCKS] }
    }

    #[inline]
    fn slot(block: usize, slot: usize, neuron: usize) -> usize {
        (block * BLOCK_SIZE + slot) * PAIRWISE_LEN + neuron
    }

    #[inline]
    fn get(&mut self, block: usize, slot: usize, neuron: usize) -> Option<u64> {
        let entry = self.entries[Self::slot(block, slot, neuron)];
        if entry.version == self.versions[block] { Some(entry.cost as u64) } else { None }
    }

    #[inline]
    fn put(&mut self, block: usize, slot: usize, neuron: usize, cost: u64) {
        self.entries[Self::slot(block, slot, neuron)] = CacheEntry { version: self.versions[block], cost: cost as u32 };
    }

    #[inline]
    fn invalidate(&mut self, block: usize) {
        self.versions[block] += 1;
    }
}

/// The two affected blocks as if `perm[i]` and `perm[j]` were swapped.
#[inline]
fn candidate_blocks<'a>(acts: &'a Activations, perm: &[usize], i: usize, j: usize) -> ([&'a [u64]; BLOCK_SIZE], [&'a [u64]; BLOCK_SIZE]) {
    let mut cols_i = acts.block(perm, i / BLOCK_SIZE);
    let mut cols_j = acts.block(perm, j / BLOCK_SIZE);
    cols_i[i % BLOCK_SIZE] = acts.col(perm[j]);
    cols_j[j % BLOCK_SIZE] = acts.col(perm[i]);
    (cols_i, cols_j)
}

/// Calibrate the start temperature to the dataset.
fn calibrate_start_temp(acts: &Activations, perm: &[usize], block_costs: &[u64], rng: &mut SmallRng) -> f64 {
    let mut uphill = Vec::with_capacity(CALIBRATION_SAMPLES);

    while uphill.len() < CALIBRATION_SAMPLES {
        let i = rng.random_range(0..PAIRWISE_LEN);
        let j = rng.random_range(0..PAIRWISE_LEN);
        if i / BLOCK_SIZE == j / BLOCK_SIZE {
            continue;
        }

        let (cols_i, cols_j) = candidate_blocks(acts, perm, i, j);
        let (new_i, new_j) = block_cost_pair(cols_i, cols_j);
        let old = block_costs[i / BLOCK_SIZE] + block_costs[j / BLOCK_SIZE];
        let delta = (new_i + new_j) as i64 - old as i64;

        if delta > 0 {
            uphill.push(delta as f64);
        }
    }

    uphill.sort_unstable_by(f64::total_cmp);
    let median = uphill[uphill.len() / 2];
    median / -START_ACCEPT.ln()
}

/// Fraction of blocks that are active.
fn nnz_ratio(acts: &Activations, cost: u64) -> f64 {
    cost as f64 / (acts.samples() * N_BLOCKS) as f64
}

fn anneal(acts: &Activations, perm: &mut [usize]) {
    let mut rng = SmallRng::seed_from_u64(0);

    let mut block_costs: Vec<u64> = (0..N_BLOCKS).map(|b| block_cost(acts.block(perm, b))).collect();
    let mut current_cost: u64 = block_costs.iter().sum();
    let initial_cost = current_cost;
    let mut cache = CostCache::new();

    let mut best_cost = current_cost;
    let mut best_perm = perm.to_vec();

    let steps = STEPS;
    let t_start = calibrate_start_temp(acts, perm, &block_costs, &mut rng);
    let t_end = t_start * END_TEMP_RATIO;
    let decay = END_TEMP_RATIO.powf(1.0 / steps as f64);
    let mut t = t_start;
    eprintln!("Annealing {} steps, T={:.0} -> {:.0}", steps, t_start, t_end);

    for step in 0..steps {
        t *= decay;

        let i = rng.random_range(0..PAIRWISE_LEN);
        let j = rng.random_range(0..PAIRWISE_LEN);

        let bi = i / BLOCK_SIZE;
        let bj = j / BLOCK_SIZE;

        // Same block.
        if bi == bj {
            continue;
        }

        let (si, sj) = (i % BLOCK_SIZE, j % BLOCK_SIZE);
        let (ni, nj) = (perm[j], perm[i]);

        let (new_i, new_j) = match (cache.get(bi, si, ni), cache.get(bj, sj, nj)) {
            // Already cached, take counts in cache.
            (Some(new_i), Some(new_j)) => (new_i, new_j),

            // Not cached, add cost to cache.
            (cached_i, cached_j) => {
                let (cols_i, cols_j) = candidate_blocks(acts, perm, i, j);

                let (new_i, new_j) = match (cached_i, cached_j) {
                    (Some(new_i), None) => (new_i, block_cost(cols_j)),
                    (None, Some(new_j)) => (block_cost(cols_i), new_j),
                    _ => block_cost_pair(cols_i, cols_j),
                };
                cache.put(bi, si, ni, new_i);
                cache.put(bj, sj, nj, new_j);
                (new_i, new_j)
            }
        };

        let old_cost = block_costs[bi] + block_costs[bj];
        let delta = (new_i + new_j) as i64 - old_cost as i64;
        let accepted = delta <= 0 || rng.random_range(0.0f64..1.0f64) < (-delta as f64 / t).exp();

        // Swap values and invalidate blocks in cache.
        if accepted {
            perm.swap(i, j);
            block_costs[bi] = new_i;
            block_costs[bj] = new_j;
            current_cost = (current_cost as i64 + delta) as u64;
            cache.invalidate(bi);
            cache.invalidate(bj);

            if current_cost < best_cost {
                best_cost = current_cost;
                best_perm.copy_from_slice(perm);
            }
        }

        if step % 1000 == 0 {
            eprintln!("Step {:>8}, T={:>9.2}, cost={}, best={}", step, t, current_cost, best_cost);
        }
    }

    perm.copy_from_slice(&best_perm);
    eprintln!("Best cost {best_cost} (nnz ratio {:.5}, from {:.5})", nnz_ratio(acts, best_cost), nnz_ratio(acts, initial_cost));
}

fn main() -> io::Result<()> {
    let acts = Activations::load_from_file(ACTS_DUMP_FILE)?;
    eprintln!("Loaded {} positions ({} samples)", acts.positions, acts.samples());

    let mut perm: Vec<usize> = (0..PAIRWISE_LEN).collect();
    anneal(&acts, &mut perm);

    let out: String = format!("{:?}", perm);
    fs::write("perm.txt", out)?;
    eprintln!("Wrote perm.txt");
    Ok(())
}
