//! Dump diagnostics about a quantised network.

use std::{io::Result, ops::Range, path::PathBuf};

use nnue::{
    arch::{EFF_L2_LEN, EmbedNNUEData, FT_QUANT, L1_LEN, L2_LEN, PAIRWISE_LEN},
    features::{NB_OUTPUT_BUCKETS, pawn::PAWN_FEATURES},
    preprocess::load_write::LoadWrite,
};

const PSQT_MAX_ACTIVE: usize = 32;

/// Rows below this mean |w| are technically nonzero but don't do much.
const WEAK_ROW: f64 = 1.0;

fn pct(part: usize, whole: usize) -> f64 {
    100.0 * part as f64 / whole as f64
}

// ----------------------------------------------------------------

fn summary_header(title: &str) {
    println!("\n--- {title} ---");
    println!(
        "{:<10} {:>10} {:>10} {:>10} {:>10} {:>9} {:>7} {:>7} {:>6}",
        "name", "count", "min", "max", "mean", "std", "zero%", "sat%", "util"
    );
}

/// min / max / mean / std, sparsity and how much of the quant range it uses.
/// `quant_bound` is the saturation point of the storage type, or `None` for full precision.
fn summary<T: Copy + Into<f64>>(name: &str, vals: &[T], quant_bound: Option<f64>) {
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut sum, mut sum2) = (0.0, 0.0);
    let (mut zero, mut sat) = (0, 0);

    for &v in vals {
        let x: f64 = v.into();
        min = min.min(x);
        max = max.max(x);
        sum += x;
        sum2 += x * x;
        zero += usize::from(x == 0.0);
        sat += usize::from(quant_bound.is_some_and(|b| x.abs() >= b));
    }

    let n = vals.len() as f64;
    let mean = sum / n;
    let std = (sum2 / n - mean * mean).max(0.0).sqrt();
    let peak = min.abs().max(max.abs());

    let (sat, util) = match quant_bound {
        Some(b) => (format!("{:>7.3}", pct(sat, vals.len())), format!("{:>6.3}", peak / b)),
        None => ("      -".to_string(), "     -".to_string()),
    };

    println!("{name:<10} {:>10} {min:>10.4} {max:>10.4} {mean:>10.4} {std:>9.4} {:>7.3} {sat} {util}", vals.len(), pct(zero, vals.len()));
}

/// Exact percentiles of |w|.
fn abs_pctiles<T: Copy + Into<f64>>(name: &str, vals: &[T], bound: usize) {
    let mut hist = vec![0u64; bound + 1];
    for &v in vals {
        hist[(v.into().abs() as usize).min(bound)] += 1;
    }

    let qs = [0.5, 0.9, 0.99, 0.999];
    let mut out = [0usize; 4];
    let (mut acc, mut qi) = (0.0, 0);

    for (i, &c) in hist.iter().enumerate() {
        acc += c as f64;
        while qi < qs.len() && acc >= qs[qi] * vals.len() as f64 {
            out[qi] = i;
            qi += 1;
        }
    }

    let max = hist.iter().rposition(|&c| c > 0).unwrap_or(0);
    println!("{name:<10} p50={:<6} p90={:<6} p99={:<6} p99.9={:<6} max={max}", out[0], out[1], out[2], out[3]);
}

// -------------------------------------------------------------

/// Mean |w| of every FT row.
fn row_weights<T: Copy + Into<f64>>(w: &[T]) -> Vec<f64> {
    w.chunks_exact(L1_LEN).map(|r| r.iter().map(|&x| x.into().abs()).sum::<f64>() / L1_LEN as f64).collect()
}

fn row_report(name: &str, rows: &[f64]) {
    let mut used: Vec<f64> = rows.iter().copied().filter(|&x| x > 0.0).collect();
    used.sort_unstable_by(f64::total_cmp);

    let q = |p: f64| used.get((used.len() as f64 * p) as usize).or(used.last()).copied().unwrap_or(0.0);
    let weak = rows.iter().filter(|&&x| x < WEAK_ROW).count();
    println!(
        "{name} {:>5}/{:<5} ({:>5.1}%)  weak {:>5.1}%  mean|w| p50={:>8.3} p90={:>8.3} p99={:>8.3} max={:>8.3}",
        used.len(),
        rows.len(),
        pct(used.len(), rows.len()),
        pct(weak, rows.len()),
        q(0.5),
        q(0.9),
        q(0.99),
        q(1.0),
    );
}

// ------------------------------------------------------------------

/// Per neuron histogram of |w| over the feature transform.
struct Hist {
    bins: usize,
    rows: usize,
    pos: Vec<u32>,
    neg: Vec<u32>,
}

impl Hist {
    fn build<T: Copy + Into<i64>>(w: &[T], bins: usize) -> Self {
        let mut h = Self { bins, rows: w.len() / L1_LEN, pos: vec![0; L1_LEN * bins], neg: vec![0; L1_LEN * bins] };

        for row in w.chunks_exact(L1_LEN) {
            for (j, &x) in row.iter().enumerate() {
                let x: i64 = x.into();
                let side = if x < 0 { &mut h.neg } else { &mut h.pos };
                side[j * h.bins + x.unsigned_abs() as usize] += 1;
            }
        }

        h
    }

    /// Sum of the `k` largest magnitudes on one side of neuron `j`.
    fn top_k(&self, neg: bool, j: usize, k: usize) -> i64 {
        let side = if neg { &self.neg } else { &self.pos };
        let (mut left, mut sum) = (k as u64, 0);

        for v in (1..self.bins).rev() {
            let take = u64::from(side[j * self.bins + v]).min(left);
            sum += take as i64 * v as i64;
            left -= take;
            if left == 0 {
                break;
            }
        }

        sum
    }

    fn nonzero(&self, j: usize) -> usize {
        self.rows - self.pos[j * self.bins] as usize
    }
}

fn max_abs<T: Copy + Into<f64>>(w: &[T]) -> usize {
    w.iter().map(|&x| x.into().abs() as usize).max().unwrap_or(0)
}

fn ft_report(nn: &EmbedNNUEData) {
    let psqt = Hist::build(&nn.ftw_psqt, max_abs(&nn.ftw_psqt) + 1);
    let thrt = Hist::build(&nn.ftw_thrt, max_abs(&nn.ftw_thrt) + 1);

    // Turn all features on at once to see if we can get out of the CReLU clamp.
    let bound = |j: usize, k: usize, neg: bool| {
        let sign = if neg { -1 } else { 1 };
        i64::from(nn.ftb[j]) + sign * (psqt.top_k(neg, j, PSQT_MAX_ACTIVE) + thrt.top_k(neg, j, k))
    };
    let all = usize::MAX;

    let dead = (0..L1_LEN).filter(|&j| psqt.nonzero(j) + thrt.nonzero(j) == 0).count();
    let always_off = (0..L1_LEN).filter(|&j| bound(j, all, false) <= 0).count();
    let always_sat = (0..L1_LEN).filter(|&j| bound(j, all, true) >= i64::from(FT_QUANT)).count();
    let dead_pairs = (0..PAIRWISE_LEN).filter(|&k| bound(k, all, false) <= 0 || bound(k + PAIRWISE_LEN, all, false) <= 0).count();

    println!("\n--- ft neurons ---");
    println!("dead (no nonzero weight)  {dead:>6}/{L1_LEN}");
    println!("always <= 0 after crelu   {always_off:>6}/{L1_LEN}");
    println!("always >= FT_QUANT        {always_sat:>6}/{L1_LEN}");
    println!("pairs with a dead half    {dead_pairs:>6}/{PAIRWISE_LEN}");
}

// ---------------------------------------------------------------------

fn main() -> Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: stats <net.bin>"));
    let nn = EmbedNNUEData::load_from_file(&path)?;

    let l1w_all: Vec<i8> = nn.l1w.iter().flat_map(|r| r.concat()).collect();
    let l2w_all: Vec<f32> = nn.l2w.iter().flat_map(|r| r.concat()).collect();

    summary_header("tensors");
    summary("ftw_psqt", &nn.ftw_psqt, Some(i16::MAX as f64));
    summary("ftw_pawn", &nn.ftw_thrt[..PAWN_FEATURES], Some(i8::MAX as f64));
    summary("ftw_thrt", &nn.ftw_thrt[PAWN_FEATURES..], Some(i8::MAX as f64));
    summary("ftb", &nn.ftb, Some(i16::MAX as f64));
    summary("l1w", &l1w_all, Some(i8::MAX as f64));
    summary("l1b", &nn.l1b.concat(), None);
    summary("l2w", &l2w_all, None);
    summary("l2b", &nn.l2b.concat(), None);
    summary("l3w", &nn.l3w.concat(), None);
    summary("l3b", &nn.l3b, None);

    println!("\n--- |w| distribution ---");
    abs_pctiles("ftw_psqt", &nn.ftw_psqt, i16::MAX as usize);
    abs_pctiles("ftw_thrt", &nn.ftw_thrt, i8::MAX as usize);
    abs_pctiles("l1w", &l1w_all, i8::MAX as usize);

    // How much of each input space the net uses.
    let psqt_rows = row_weights(&nn.ftw_psqt);
    let thrt_rows = row_weights(&nn.ftw_thrt);
    println!("\n--- feature rows (weak: mean |w| < {WEAK_ROW}) ---");
    row_report("psqt", &psqt_rows);
    row_report("pawn", &thrt_rows[..PAWN_FEATURES]);
    row_report("thrt", &thrt_rows[PAWN_FEATURES..]);

    ft_report(&nn);

    println!("\n--- l1 / output path (per bucket) ---");
    println!("{:<4} {:>11} {:>9} {:>10} {:>10} {:>10} {:>7}", "bkt", "dead rows", "dead cols", "l3b", "skip |w|", "deep |w|", "skip%");
    for b in 0..NB_OUTPUT_BUCKETS {
        let dead_rows = nn.l1w.iter().filter(|r| r[b].iter().all(|&w| w == 0)).count();
        let dead_cols = (0..L2_LEN).filter(|&j| nn.l1w.iter().all(|r| r[b][j] == 0)).count();

        let mass = |r: Range<usize>| r.map(|i| f64::from(nn.l3w[i][b]).abs()).sum::<f64>();
        let (skip, deep) = (mass(0..EFF_L2_LEN), mass(EFF_L2_LEN..nn.l3w.len()));

        println!(
            "{b:<4} {:>5}/{L1_LEN:<5} {:>3}/{L2_LEN:<5} {:>10.4} {skip:>10.4} {deep:>10.4} {:>6.1}%",
            dead_rows,
            dead_cols,
            nn.l3b[b],
            100.0 * skip / (skip + deep)
        );
    }

    Ok(())
}
