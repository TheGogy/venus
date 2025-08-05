use std::{
    array,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
};

use chess::{
    defs::MAX_PLY,
    types::{Depth, board::Board, color::Color, eval::Eval, moves::Move},
};

use crate::{
    history::{
        capturehist::CaptureHist,
        conthist::{CONT_NUM, ContHist, PieceTo},
        corrhist::{CorrHist, correction_bonus},
        hist_delta,
        movebuffer::MoveBuffer,
        quiethist::QuietHist,
    },
    time_management::{timecontrol::TimeControl, timemanager::TimeManager},
    tunables::params::tunables::{hist_corr_other, hist_corr_pawn},
};

use super::{pv::PVLine, stack::SearchStackEntry};

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
    pub hist_noisy: CaptureHist,
    pub hist_conts: [ContHist; CONT_NUM],
    pub hist_corr_pawn: CorrHist,
    pub hist_corr_major_w: CorrHist,
    pub hist_corr_major_b: CorrHist,
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
            hist_noisy: CaptureHist::default(),
            hist_conts: array::from_fn(|_| ContHist::default()),

            hist_corr_pawn: CorrHist::default(),
            hist_corr_major_w: CorrHist::default(),
            hist_corr_major_b: CorrHist::default(),
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

    /// Whether our opponent's position is getting worse.
    pub fn opp_worsening(&self) -> bool {
        self.ply >= 1 && self.ss_at(1).eval + self.ss().eval > Eval(1)
    }
}

/// Histories.
impl Thread {
    /// Get the piece and square of the move n steps back.
    pub const fn pieceto_at(&self, offset: usize) -> Option<PieceTo> {
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

    #[rustfmt::skip]
    /// Get the correction score for a given board position according to our correction history.
    pub fn correction_score(&self, b: &Board) -> Eval {
        let key = b.state.hash;

        Eval (
            hist_corr_pawn()  * self.hist_corr_pawn.get_bonus(key.pawn_key, b.stm)                            / 1024 +
            hist_corr_other() * self.hist_corr_major_w.get_bonus(key.non_pawn_key[Color::White.idx()], b.stm) / 1024 +
            hist_corr_other() * self.hist_corr_major_b.get_bonus(key.non_pawn_key[Color::Black.idx()], b.stm) / 1024
        )
    }

    /// Update the correction history.
    pub fn update_corrhist(&mut self, b: &Board, best_value: Eval, depth: Depth) {
        let key = b.state.hash;
        let bonus = correction_bonus(best_value, self.ss().eval, depth);

        self.hist_corr_pawn.add_bonus(key.pawn_key, b.stm, bonus);
        self.hist_corr_major_w.add_bonus(key.non_pawn_key[Color::White.idx()], b.stm, bonus);
        self.hist_corr_major_b.add_bonus(key.non_pawn_key[Color::Black.idx()], b.stm, bonus);
    }
}
