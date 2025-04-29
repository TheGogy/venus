use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

use chess::types::{eval::Eval, moves::Move};

use crate::{search::pv::PVLine, timeman::clock::Clock};

#[derive(Clone, Debug)]
pub struct Thread {
    pub clock: Clock,

    pub eval: Eval,

    pub ply: usize,
    pub depth: usize,
    pub seldepth: usize,
    pub ply_from_null: usize,
    pub nodes: u64,

    pub pv: PVLine,
    pub stop: bool,
}

impl Thread {
    /// Creates a new thread.
    pub fn new(clock: Clock) -> Self {
        Self { clock, eval: Eval::DRAW, ply: 0, depth: 0, seldepth: 0, ply_from_null: 0, nodes: 0, pv: PVLine::default(), stop: false }
    }

    /// Creates a new idle thread.
    pub fn idle(global_stop: Arc<AtomicBool>, global_nodes: Arc<AtomicU64>) -> Self {
        Self::new(Clock::wait(global_stop, global_nodes))
    }

    /// Whether we should start the next iteration.
    #[inline]
    pub fn should_start_iter(&mut self) -> bool {
        self.clock.should_start_iteration(self.depth, self.nodes, self.best_move())
    }

    /// Whether we should stop searching.
    #[inline]
    pub fn should_stop(&mut self) -> bool {
        self.stop || self.clock.should_stop(self.nodes)
    }

    /// The best move found by this thread.
    #[inline]
    pub const fn best_move(&self) -> Move {
        self.pv.moves[0]
    }

    /// Prepare this thread to search.
    pub fn prepare_search(&mut self, halfmoves: usize) {
        self.clock.prepare_search();
        self.ply = 0;
        self.depth = 0;
        self.seldepth = 0;
        self.ply_from_null = halfmoves;
        self.nodes = 0;
    }

    /// Tell the thread that a move has been made.
    #[inline]
    pub const fn move_made(&mut self) {
        self.ply += 1;
        self.ply_from_null += 1;
        self.nodes += 1;
    }

    /// Tell the thread that a move has been undone.
    #[inline]
    pub const fn move_undo(&mut self) {
        self.ply -= 1;
        self.ply_from_null -= 1;
    }
}
