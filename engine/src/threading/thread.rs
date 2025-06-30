use std::{
    array,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
};

use chess::{
    Depth, MAX_PLY,
    types::{board::Board, color::Color, eval::Eval, moves::Move},
};

use crate::{
    history::{
        conthist::{CONT_NUM, ContHist, PieceTo},
        hist_delta,
        movebuffer::MoveBuffer,
        noisyhist::NoisyHist,
        quiethist::QuietHist,
    },
    search::pv::PVLine,
    time_management::{timecontrol::TimeControl, timemanager::TimeManager},
};

use super::stack::SearchStackEntry;

#[derive(Clone, Debug)]
pub struct Thread {
    // Time management.
    pub tm: TimeManager,
    pub stop: bool,

    // Search data.
    pub ply: usize,
    pub depth: Depth,
    pub seldepth: usize,
    pub ply_from_null: usize,
    pub nodes: u64,
    pub eval: Eval,
    pub pv: PVLine,
    pub stack: [SearchStackEntry; MAX_PLY],

    // Histories.
    pub hist_quiet: QuietHist,
    pub hist_noisy: NoisyHist,
    pub hist_conts: [ContHist; CONT_NUM],
}

impl Thread {
    /// Creates a new thread.
    pub fn new(tm: TimeManager) -> Self {
        Self {
            tm,
            stop: false,

            ply: 0,
            depth: 0,
            seldepth: 0,
            ply_from_null: 0,
            nodes: 0,
            eval: Eval::DRAW,
            pv: PVLine::default(),
            stack: [SearchStackEntry::default(); MAX_PLY],

            hist_quiet: QuietHist::default(),
            hist_noisy: NoisyHist::default(),
            hist_conts: array::from_fn(|_| ContHist::default()),
        }
    }

    /// Creates a new idle thread.
    pub fn idle(global_stop: Arc<AtomicBool>, global_nodes: Arc<AtomicU64>) -> Self {
        Self::new(TimeManager::new(global_stop, global_nodes, TimeControl::Infinite, Color::White))
    }

    /// Creates a new placeholder thread. Used for testing.
    pub fn placeholder() -> Self {
        Self::new(TimeManager::new(Arc::new(AtomicBool::new(false)), Arc::new(AtomicU64::new(0)), TimeControl::Infinite, Color::White))
    }

    pub fn fixed_depth(depth: Depth) -> Self {
        Self::new(TimeManager::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicU64::new(0)),
            TimeControl::FixedDepth(depth),
            Color::White,
        ))
    }

    /// Whether we should start the next iteration.
    pub fn should_start_iter(&mut self) -> bool {
        self.depth < MAX_PLY as Depth && self.tm.should_start_iter(self.depth + 1, self.nodes, self.best_move())
    }

    /// Whether we should stop searching.
    pub fn should_stop(&mut self) -> bool {
        self.stop || !self.tm.should_continue(self.nodes)
    }

    /// The best move found by this thread.
    pub const fn best_move(&self) -> Move {
        self.pv.moves[0]
    }

    /// Prepare this thread to search.
    pub fn prepare_search(&mut self, halfmoves: usize) {
        self.tm.prepare_search();
        self.ply = 0;
        self.depth = 0;
        self.seldepth = 0;
        self.ply_from_null = halfmoves;
        self.nodes = 0;
        self.stop = false;
    }

    /// Tell the thread that a move has been made.
    pub const fn move_made(&mut self, pt: PieceTo) {
        self.ss_mut().pieceto = Some(pt);
        self.ss_mut().ply_from_null = self.ply_from_null;

        self.ply += 1;
        self.ply_from_null += 1;
        self.nodes += 1;
    }

    /// Tell the thread that a null move has been made.
    pub const fn null_made(&mut self) {
        self.ss_mut().pieceto = None;
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
    pub fn is_improving(&self) -> bool {
        if self.ply >= 2 && self.ss_at(2).eval != -Eval::INFINITY {
            self.ss().eval > self.ss_at(2).eval
        } else if self.ply >= 4 && self.ss_at(4).eval != -Eval::INFINITY {
            self.ss().eval > self.ss_at(4).eval
        } else {
            true
        }
    }
}

/// Histories.
impl Thread {
    /// Get the piece and square of the move n steps back.
    pub fn pieceto_at(&self, offset: usize) -> Option<PieceTo> {
        if self.ply >= offset {
            return self.ss_at(offset).pieceto;
        }
        None
    }

    /// Gets the previous moves played in the position for the continuation history.
    pub fn get_prev_piecetos(&self) -> [Option<PieceTo>; CONT_NUM] {
        let mut pms = [None; CONT_NUM];

        for (i, pm) in pms.iter_mut().enumerate() {
            *pm = self.pieceto_at(i + 1);
        }

        pms
    }

    /// Update the history tables given some quiet and noisy moves.
    pub fn update_history(&mut self, best: Move, depth: Depth, board: &Board, quiets: &MoveBuffer, captures: &MoveBuffer) {
        let (bonus, malus) = hist_delta(depth);
        self.hist_noisy.update(board, best, captures, bonus, malus);

        if best.flag().is_quiet() {
            self.hist_quiet.update(board.stm, best, quiets, bonus, malus);

            for i in 0..CONT_NUM {
                if let Some(pt) = self.pieceto_at(i + 1) {
                    self.hist_conts[i].update(best, pt, quiets, bonus, malus);
                }
            }
        }
    }

    /// Get the history score for a given move.
    pub fn hist_score(&self, b: &Board, m: Move) -> i32 {
        if m.flag().is_cap() {
            self.hist_noisy.get_bonus(b, m)
        } else {
            let mut v = self.hist_quiet.get_bonus(b.stm, m);
            for i in 0..CONT_NUM {
                if let Some(pt) = self.pieceto_at(i + 1) {
                    v += self.hist_conts[i].get_bonus(m, pt);
                }
            }
            v
        }
    }
}

impl Thread {
    pub fn score_cap_hist(&self, m: Move, board: &Board) -> i32 {
        self.hist_noisy.get_bonus(board, m)
    }

    pub fn assign_history_scores(&self, side: Color, moves: &[Move], scores: &mut [i32]) {
        for i in 0..moves.len() {
            scores[i] = self.hist_quiet.get_bonus(side, moves[i]);
        }

        for i in 0..CONT_NUM {
            if let Some(entry) = self.get_previous_entry(1 + i) {
                let pt = entry.pieceto.unwrap();
                for j in 0..moves.len() {
                    scores[j] += self.hist_conts[i].get_bonus(moves[j], pt);
                }
            }
        }
    }

    fn get_previous_entry(&self, rollback: usize) -> Option<SearchStackEntry> {
        if self.ply >= rollback && self.ss_at(rollback).pieceto.is_some() {
            Some(*self.ss_at(rollback))
        } else {
            None
        }
    }
}
