use std::time::Instant;

use crate::{interface::Engine, position::pos::Pos, threading::thread::Thread, timeman::clock::Clock};

// TODO: Increase this when we have pruning
const BENCH_DEPTH: usize = 4;

impl Engine {
    /// Runs a benchmark of the engine on a number of positions.
    pub fn run_bench() {
        let mut total_nodes = 0;
        let mut total_time = 0;

        for fen in FENS {
            let mut pos: Pos = format!("fen {fen}").parse().unwrap();
            let mut thread = Thread::new(Clock::fixed_depth(BENCH_DEPTH));

            let start = Instant::now();
            pos.iterative_deepening::<false>(&mut thread);

            total_time += start.elapsed().as_micros();
            total_nodes += thread.nodes;
        }

        println!("{total_nodes} nodes {} nps", total_nodes * 1_000_000 / (total_time as u64).max(1))
    }
}

const FENS: &[&str] = &["1r2r2k/1b4q1/pp5p/2pPp1p1/P3Pn2/1P1B1Q1P/2R3P1/4BR1K b - - 1 37"];

