use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use chess::{
    Depth,
    types::{color::Color, moves::Move, square::Square},
};

use super::timecontrol::TimeControl;

#[derive(Clone, Debug)]
pub struct TimeManager {
    // Constructed at start.
    start: Instant,
    tc: TimeControl,
    opt: Duration,
    max: Duration,

    // Shared between all threads.
    global_stop: Arc<AtomicBool>,
    global_nodes: Arc<AtomicU64>,

    // Thread-specific.
    last_check: u64,
    move_nodes: [[u64; Square::NUM]; Square::NUM],
}

/// Display time used in UCI format.
impl fmt::Display for TimeManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elapsed = self.elapsed().as_millis().max(1);
        let nodes = self.global_nodes();
        let nps = (nodes as u128 * 1000) / elapsed;

        write!(f, "nodes {nodes} nps {nps} time {elapsed}")
    }
}

impl TimeManager {
    const FREQUENCY: u64 = 2048;

    /// Initialize a new time manager.
    pub fn new(global_stop: Arc<AtomicBool>, global_nodes: Arc<AtomicU64>, tc: TimeControl, stm: Color) -> Self {
        let (opt, max) = tc.opt_max_time(stm);
        let start = Instant::now();

        Self { start, tc, opt, max, global_stop, global_nodes, last_check: 0, move_nodes: [[0; Square::NUM]; Square::NUM] }
    }

    /// Initialize a new time manager that only searches to a fixed depth.
    pub fn fixed_depth(depth: Depth) -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), Arc::new(AtomicU64::new(0)), TimeControl::FixedDepth(depth), Color::White)
    }

    /// Whether we should start the given iteration.
    pub fn should_start_iter(&mut self, depth: Depth, nodes: u64, best_move: Move) -> bool {
        if self.is_stopped() {
            return false;
        }

        if depth <= 1 {
            return true;
        }

        let should_start = match self.tc {
            // Non time related time controls (opt and max unset).
            TimeControl::Infinite => true,
            TimeControl::FixedDepth(d) => depth <= d,
            TimeControl::FixedNodes(n) => self.global_nodes() <= n,

            // Time related time controls (opt and max set).
            _ => {
                let scale = if !best_move.is_none() && nodes != 0 {
                    let f = self.move_nodes[best_move.src().idx()][best_move.dst().idx()] as f64 / nodes as f64;

                    (0.4 + (1.0 - f) * 2.0).max(0.5)
                } else {
                    1.0
                };

                self.elapsed() < self.opt.mul_f64(scale)
            }
        };

        // If we should stop, tell the other threads to also stop.
        if !should_start {
            self.raise_stop();
        }

        should_start
    }

    /// Whether we should continue an ongoing search.
    pub fn should_continue(&mut self, nodes: u64) -> bool {
        let delta = nodes - self.last_check;

        if delta >= Self::FREQUENCY {
            self.global_nodes.fetch_add(delta, Ordering::Relaxed);
            self.last_check = nodes;
            if self.is_stopped() {
                return false;
            }
        }

        let should_continue = match self.tc {
            TimeControl::Variable { .. } | TimeControl::FixedTime(_) => delta < Self::FREQUENCY || self.elapsed() < self.max,
            _ => true,
        };

        if !should_continue {
            self.raise_stop();
        }

        should_continue
    }
}

// Helper methods.
impl TimeManager {
    /// Whether the stop flag has been raised.
    pub fn is_stopped(&self) -> bool {
        self.global_stop.load(Ordering::Relaxed)
    }

    /// The total nodes searched across all threads.
    pub fn global_nodes(&self) -> u64 {
        self.global_nodes.load(Ordering::Relaxed)
    }

    /// The total elapsed time since we started searching.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Update the node count for the given move.
    pub const fn update_nodes(&mut self, m: Move, nodes_searched: u64) {
        self.move_nodes[m.src().idx()][m.dst().idx()] += nodes_searched;
    }

    /// Raise the stop flag to tell all threads to stop searching now.
    pub fn raise_stop(&mut self) {
        self.global_stop.store(true, Ordering::Relaxed);
    }

    /// Prepare the timemanager for a search.
    pub fn prepare_search(&mut self) {
        self.last_check = 0;
    }
}
