use std::{
    array,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
};

use chess::{
    MAX_DEPTH,
    types::{board::Board, eval::Eval, moves::Move, piece::CPiece},
};
use nnue::network::NNUE;

use crate::{
    history::{
        conthist::{CONT_NUM, ContHist, PieceTo},
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
    pub depth: i16,
    pub seldepth: usize,
    pub ply_from_null: usize,
    pub nodes: u64,

    pub hist_quiet: QuietHist,
    pub hist_noisy: NoisyHist,
    pub hist_conts: [ContHist; CONT_NUM],

    pub pv: PVLine,
    pub stack: [SearchStackEntry; MAX_DEPTH],

    pub nnue: NNUE,
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
            hist_conts: array::from_fn(|_| ContHist::default()),
            pv: PVLine::default(),
            stack: [SearchStackEntry::default(); MAX_DEPTH],
            nnue: NNUE::default(),
        }
    }

    /// Creates a new idle thread.
    pub fn idle(global_stop: Arc<AtomicBool>, global_nodes: Arc<AtomicU64>) -> Self {
        Self::new(Clock::wait(global_stop, global_nodes))
    }

    /// Creates a new placeholder thread. Used for testing.
    pub fn placeholder() -> Self {
        Self::new(Clock::fixed_depth(0))
    }

    /// Whether we should start the next iteration.
    pub fn should_start_iter(&mut self) -> bool {
        self.clock.should_start_iteration(self.depth + 1, self.nodes, self.best_move())
    }

    /// Whether we should stop searching.
    pub fn should_stop(&mut self) -> bool {
        self.stop || self.clock.should_stop(self.nodes)
    }

    /// The best move found by this thread.
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
    pub const fn move_made(&mut self, p: CPiece, m: Move) {
        self.ss_mut().mvp = p;
        self.ss_mut().mov = m;
        self.ss_mut().ply_from_null = self.ply_from_null;

        self.ply += 1;
        self.ply_from_null += 1;
        self.nodes += 1;
    }

    /// Tell the thread that a null move has been made.
    pub const fn null_made(&mut self) {
        self.ss_mut().mvp = CPiece::None;
        self.ss_mut().mov = Move::NULL;
        self.ss_mut().ply_from_null = self.ply_from_null;

        self.ply += 1;
        self.ply_from_null = 0;
        self.nodes += 1;
    }

    /// Tell the thread that a move has been undone.
    pub const fn move_undo(&mut self) {
        self.ply -= 1;
        self.ply_from_null = self.ss().ply_from_null;
    }

    /// Whether the current position is improving.
    pub fn is_improving(&self, in_check: bool) -> bool {
        if in_check {
            false
        } else if self.ply >= 2 && self.ss_at(2).eval != -Eval::INFINITY {
            self.ss().eval > self.ss_at(2).eval
        } else if self.ply >= 4 && self.ss_at(4).eval != -Eval::INFINITY {
            self.ss().eval > self.ss_at(4).eval
        } else {
            true
        }
    }
}

impl Thread {
    /// Get the piece and square of the move n steps back.
    pub fn pieceto_at(&self, offset: usize) -> Option<PieceTo> {
        if self.ply >= offset {
            let se = self.ss_at(offset);

            if se.mov.is_valid() {
                return Some((se.mvp, se.mov.dst()));
            }
        }
        None
    }

    /// Gets the previous moves played in the position for the continuation history.
    pub fn get_prev_moves(&self) -> [Option<PieceTo>; CONT_NUM] {
        let mut pms = [None; CONT_NUM];

        for (i, pm) in pms.iter_mut().enumerate() {
            *pm = self.pieceto_at(i + 1);
        }

        pms
    }

    /// Update the history tables given some quiet and noisy moves.
    pub fn update_history(&mut self, best: Move, depth: i16, board: &Board, quiets: Vec<Move>, noisies: Vec<Move>) {
        let (bonus, malus) = hist_delta(depth);
        self.hist_noisy.update(board, best, &noisies, bonus, malus);

        if best.flag().is_quiet() {
            self.hist_quiet.update(board.stm, best, &quiets, bonus, malus);

            for i in 0..CONT_NUM {
                if let Some(pt) = self.pieceto_at(i + 1) {
                    self.hist_conts[i].update(best, pt, &quiets, bonus, malus);
                }
            }
        }
    }
}
