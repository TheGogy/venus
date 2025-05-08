use std::{
    array,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
};

use chess::{
    MAX_DEPTH,
    types::{board::Board, color::Color, eval::Eval, moves::Move, piece::CPiece, square::Square},
};

use crate::{
    history::{
        conthist::{CONT_NUM, ContHist},
        hist_delta,
        noisyhist::NoisyHist,
        quiethist::QuietHist,
    },
    search::{pv::PVLine, stack::SearchStackEntry},
    timeman::clock::Clock,
};

#[derive(Clone, Debug)]
pub struct Thread {
    pub clock: Clock,
    pub stop: bool,

    pub eval: Eval,

    pub ply: usize,
    pub depth: usize,
    pub seldepth: usize,
    pub ply_from_null: usize,
    pub nodes: u64,

    pub hist_quiet: QuietHist,
    pub hist_noisy: NoisyHist,
    pub hist_cont: [ContHist; CONT_NUM],

    pub pv: PVLine,
    pub stack: [SearchStackEntry; MAX_DEPTH],
}

impl Thread {
    /// Creates a new thread.
    pub fn new(clock: Clock) -> Self {
        Self {
            clock,
            eval: Eval::DRAW,
            ply: 0,
            depth: 0,
            seldepth: 0,
            ply_from_null: 0,
            nodes: 0,
            stop: false,
            hist_quiet: QuietHist::default(),
            hist_noisy: NoisyHist::default(),
            hist_cont: array::from_fn(|_| ContHist::default()),
            pv: PVLine::default(),
            stack: [SearchStackEntry::default(); MAX_DEPTH],
        }
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
        self.stop = false;
    }

    /// Tell the thread that a move has been made.
    #[inline]
    pub const fn move_made(&mut self, p: CPiece, m: Move) {
        self.ss_mut().mvp = p;
        self.ss_mut().mov = m;
        self.ss_mut().ply_from_null = self.ply_from_null;

        self.ply += 1;
        self.ply_from_null += 1;
        self.nodes += 1;
    }

    /// Tell the thread that a null move has been made.
    #[inline]
    pub const fn null_made(&mut self) {
        self.ss_mut().mvp = CPiece::None;
        self.ss_mut().mov = Move::NULL;
        self.ss_mut().ply_from_null = self.ply_from_null;

        self.ply += 1;
        self.ply_from_null = 0;
        self.nodes += 1;
    }

    /// Tell the thread that a move has been undone.
    #[inline]
    pub const fn move_undo(&mut self) {
        self.ply -= 1;
        self.ply_from_null = self.ss().ply_from_null;
    }
}

impl Thread {
    /// Get the piece and square of the previous move, if it exists.
    fn prev_move(&self, offset: usize) -> Option<(CPiece, Square)> {
        if self.ply >= offset && !self.ss_at(offset).mov.is_null() {
            let se = self.ss_at(offset);
            Some((se.mvp, se.mov.tgt()))
        } else {
            None
        }
    }

    /// Update the history tables given some quiet and noisy moves.
    pub fn update_tables(&mut self, best: Move, depth: usize, board: &Board, quiets: Vec<Move>, noisies: Vec<Move>) {
        let (bonus, malus) = hist_delta(depth);
        self.hist_noisy.update(board, best, &noisies, bonus, malus);

        if best.flag().is_quiet() {
            self.hist_quiet.update(board.stm, best, &quiets, bonus, malus);

            for i in 0..CONT_NUM {
                if let Some((p, tgt)) = self.prev_move(1 + i) {
                    self.hist_cont[i].update(best, p, tgt, &quiets, bonus, malus);
                }
            }
        }
    }

    pub fn assign_history_scores(&self, c: Color, moves: &[Move], scores: &mut [i32]) {
        for i in 0..moves.len() {
            scores[i] = self.hist_quiet.get_bonus(c, moves[i]);
        }

        for i in 0..CONT_NUM {
            if let Some((piece, tgt)) = self.prev_move(1 + i) {
                for j in 0..moves.len() {
                    scores[j] += self.hist_cont[i].get_bonus(moves[j], piece, tgt);
                }
            }
        }
    }
}
