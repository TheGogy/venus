use std::{
    collections::HashMap,
    iter,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
};

use chess::types::moves::Move;

use crate::{
    position::Position,
    time_management::{timecontrol::TimeControl, timemanager::TimeManager},
    tt::table::TT,
};

use super::thread::Thread;

/// ThreadPool struct.
/// Contains all the threads used for searching.
pub struct ThreadPool {
    main: Thread,
    workers: Vec<Thread>,
    global_stop: Arc<AtomicBool>,
    global_nodes: Arc<AtomicU64>,
}

/// ThreadPool management.
impl ThreadPool {
    /// Initialize a threadpool.
    pub fn new(global_stop: Arc<AtomicBool>) -> Self {
        let global_nodes = Arc::new(AtomicU64::new(0));
        Self { main: Thread::idle(global_stop.clone(), global_nodes.clone()), workers: Vec::new(), global_stop, global_nodes }
    }

    /// Resize the threadpool to `n` workers.
    pub fn resize(&mut self, new_len: usize) {
        self.main = Thread::idle(self.global_stop.clone(), self.global_nodes.clone());
        self.workers.resize_with(new_len, || Thread::idle(self.global_stop.clone(), self.global_nodes.clone()));
    }

    /// Reset all threads in the threadpool.
    pub fn reset(&mut self) {
        self.resize(self.workers.len());
    }

    /// The total number of thread workers.
    pub const fn nb_workers(&self) -> usize {
        self.workers.len()
    }
}

/// Searching.
impl ThreadPool {
    /// Starts searching the given position.
    pub fn go(&mut self, pos: &mut Position, tc: TimeControl, tt: &TT) -> Move {
        self.setup_threads(pos, tc);
        self.deploy_threads(pos, tt);

        self.select_move()
    }

    /// Sets up the threads.
    fn setup_threads(&mut self, pos: &mut Position, tc: TimeControl) {
        let halfmoves = pos.board.state.halfmoves;

        self.main.tm = TimeManager::new(self.global_stop.clone(), self.global_nodes.clone(), tc, pos.board.stm);

        // Prepare main thread.
        self.main.prepare_search(halfmoves);

        // Prepare workers.
        self.workers.iter_mut().for_each(|t| t.prepare_search(halfmoves));

        // Store limits.
        self.global_stop.store(false, Ordering::Relaxed);
        self.global_nodes.store(0, Ordering::Relaxed);
    }

    /// Deploys all threads searching in the given position.
    fn deploy_threads(&mut self, pos: &mut Position, tt: &TT) {
        thread::scope(|scope| {
            for worker in &mut self.workers {
                let mut worker_pos = pos.clone();
                scope.spawn(move || {
                    worker_pos.iterative_deepening::<false>(worker, tt);
                });
            }

            pos.iterative_deepening::<true>(&mut self.main, tt);
            self.global_stop.store(true, Ordering::Relaxed);
        });
    }

    /// Selects the best move from all the threads after they have searched.
    fn select_move(&self) -> Move {
        let all_threads = iter::once(&self.main).chain(self.workers.iter());
        let max_depth = all_threads.clone().map(|thread| thread.depth).max().unwrap_or(0);

        // Count votes from all the threads at the max depth.
        let move_counts = all_threads.filter(|thread| thread.depth == max_depth).map(|thread| thread.best_move()).fold(
            HashMap::new(),
            |mut counts, mv| {
                *counts.entry(mv).or_insert(0) += 1;
                counts
            },
        );

        // Select the move with the highest count.
        move_counts.into_iter().max_by_key(|&(_, count)| count).map(|(mv, _)| mv).unwrap_or(Move::NONE)
    }
}
