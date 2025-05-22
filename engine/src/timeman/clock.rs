use core::fmt;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use chess::{
    MAX_DEPTH,
    types::{color::Color, moves::Move, square::Square},
};

use super::time_control::{TimeControl, TimeSettings};

const FREQUENCY: u64 = 2048;
const OVERHEAD: u64 = 50;

/// Clock.
/// This coordinates the threads and tells each thread when to stop.
#[derive(Clone, Debug)]
pub struct Clock {
    global_stop: Arc<AtomicBool>,
    global_nodes: Arc<AtomicU64>,
    config: ClockConfig,
    start_time: Instant,
    node_count: [[u64; Square::NUM]; Square::NUM],
    last_node_check: u64,
}

/// Clock config.
/// This contains the optimal and max times.
#[derive(Clone, Debug)]
struct ClockConfig {
    control: TimeControl,
    opt: Duration,
    max: Duration,
}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elapsed = self.elapsed().as_millis().max(1);
        let nodes = self.global_nodes();
        write!(f, "nodes {} nps {} time {}", nodes, (nodes as u128 * 1000) / elapsed, elapsed)
    }
}

impl Clock {
    /// Get the total number of nodes searched by all threads.
    #[inline]
    pub fn global_nodes(&self) -> u64 {
        self.global_nodes.load(Ordering::Relaxed)
    }

    /// Get the time elapsed since starting the search.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Whether we have stopped.
    #[inline]
    pub fn is_stopped(&self) -> bool {
        self.global_stop.load(Ordering::Relaxed)
    }

    /// Update the node count for a given move.
    #[inline]
    pub fn update_node_count(&mut self, m: Move, searched: u64) {
        self.node_count[m.src().idx()][m.dst().idx()] += searched;
    }

    /// Spawn a generic clock for threads that should idle.
    pub fn wait(global_stop: Arc<AtomicBool>, global_nodes: Arc<AtomicU64>) -> Self {
        Self::new(global_stop, global_nodes, TimeControl::Infinite, Color::White)
    }

    /// Spawn a clock that searches up to a fixed depth.
    pub fn fixed_depth(depth: i16) -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), Arc::new(AtomicU64::new(0)), TimeControl::FixedDepth(depth), Color::White)
    }

    /// Prepare clock for a search.
    pub fn prepare_search(&mut self) {
        self.start_time = Instant::now();
        self.last_node_check = 0;
        self.node_count = [[0; Square::NUM]; Square::NUM];
        self.global_nodes.store(0, Ordering::Relaxed);
        self.global_stop.store(false, Ordering::Relaxed);
    }

    pub fn new(global_stop: Arc<AtomicBool>, global_nodes: Arc<AtomicU64>, tc: TimeControl, c: Color) -> Self {
        let config = ClockConfig::new(tc, c);

        Self {
            global_stop,
            global_nodes,
            config,
            start_time: Instant::now(),
            node_count: [[0; Square::NUM]; Square::NUM],
            last_node_check: 0,
        }
    }
}

/// Whether we should start or continue the search.
impl Clock {
    /// Whether we should start the search.
    pub fn should_start_iteration(&mut self, depth: i16, nodes: u64, best_move: Move) -> bool {
        if self.is_stopped() || !(1..MAX_DEPTH as i16).contains(&depth) {
            return depth == 0;
        }

        let continue_search = match self.config.control {
            TimeControl::Infinite => true,
            TimeControl::FixedDepth(d) => depth <= d,
            TimeControl::FixedNodes(n) => self.global_nodes() <= n,
            TimeControl::FixedTime(_) | TimeControl::Variable(_) => {
                let scale = self.calculate_time_scale(best_move, nodes);
                self.elapsed() < self.config.opt.mul_f64(scale)
            }
        };

        if !continue_search {
            self.global_stop.store(true, Ordering::Relaxed);
        }
        continue_search
    }

    /// Calculate the time scale.
    const fn calculate_time_scale(&self, best_move: Move, nodes: u64) -> f64 {
        if best_move.is_null() || nodes == 0 {
            return 1.0;
        }

        let best_move_nodes = self.node_count[best_move.src().idx()][best_move.dst().idx()];
        let ratio = best_move_nodes as f64 / nodes as f64;
        (0.4 + (1.0 - ratio) * 2.0).max(0.5)
    }

    /// Whether we should stop the search.
    pub fn should_stop(&mut self, nodes: u64) -> bool {
        let delta = nodes - self.last_node_check;

        if delta >= FREQUENCY {
            self.global_nodes.fetch_add(delta, Ordering::Relaxed);
            self.last_node_check = nodes;
            if self.is_stopped() {
                return true;
            }
        }

        let stop = match self.config.control {
            TimeControl::FixedTime(_) | TimeControl::Variable(_) => delta >= FREQUENCY && self.elapsed() >= self.config.max,
            _ => false,
        };

        if stop {
            self.global_stop.store(true, Ordering::Relaxed);
        }

        stop
    }
}

impl ClockConfig {
    /// Create a new ClockConfig from time control settings.
    fn new(control: TimeControl, side: Color) -> Self {
        let (opt, max) = match control {
            TimeControl::FixedTime(ms) => {
                let time = Duration::from_millis(ms.saturating_sub(OVERHEAD));
                (time, time)
            }
            TimeControl::Variable(settings) => {
                let (time, inc) = Self::get_time_and_inc(side, &settings);
                Self::calculate_time(time, inc, settings.movestogo)
            }
            _ => (Duration::ZERO, Duration::ZERO),
        };

        Self { control, opt, max }
    }

    /// Get the time and increment for our side.
    fn get_time_and_inc(side: Color, settings: &TimeSettings) -> (u64, u64) {
        let (time, inc) = match side {
            Color::White => (settings.wtime, settings.winc.unwrap_or(0)),
            Color::Black => (settings.btime, settings.binc.unwrap_or(0)),
        };
        (time.saturating_sub(FREQUENCY), inc)
    }

    /// Calculate the optimal and max time for the given time control.
    fn calculate_time(time: u64, inc: u64, moves: Option<u64>) -> (Duration, Duration) {
        let (opt, max) = if let Some(moves) = moves {
            let scale = 0.7 / (moves.min(50) as f64);
            let base = scale * time as f64;
            let ceiling = 0.8 * time as f64;
            (base.min(ceiling), (5.0 * base).min(ceiling))
        } else {
            let base = ((time / 20) + (inc * 3 / 4)) as f64;
            (0.6 * base, (2.0 * base).min(time as f64))
        };

        (Duration::from_millis(opt as u64), Duration::from_millis(max as u64))
    }
}
