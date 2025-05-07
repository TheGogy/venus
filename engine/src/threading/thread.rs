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
    search::pv::PVLine,
    timeman::clock::Clock,
};

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

    history: QuietHist,
    caphist: NoisyHist,
    conthists: [ContHist; CONT_NUM],

    pub stack: [SearchStackEntry; MAX_DEPTH],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchStackEntry {
    ply_from_null: usize,
    moved: Option<CPiece>,
    move_made: Option<Move>,
    pub eval: Eval,
    pub excluded: Option<Move>,
    pub ttpv: bool,
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
            pv: PVLine::default(),
            stop: false,
            history: QuietHist::default(),
            caphist: NoisyHist::default(),
            conthists: array::from_fn(|_| ContHist::default()),
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
        self.pv = PVLine::default();
        self.stop = false;
    }

    /// Tell the thread that a move has been made.
    #[inline]
    pub const fn move_made(&mut self, p: CPiece, m: Move) {
        self.ss_mut().moved = Some(p);
        self.ss_mut().move_made = Some(m);
        self.ss_mut().ply_from_null = self.ply_from_null;

        self.ply += 1;
        self.ply_from_null += 1;
        self.nodes += 1;
    }

    /// Tell the thread that a null move has been made.
    #[inline]
    pub const fn null_made(&mut self) {
        self.ss_mut().moved = None;
        self.ss_mut().move_made = None;
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
    pub const fn ss(&self) -> &SearchStackEntry {
        &self.stack[self.ply]
    }

    pub const fn ss_mut(&mut self) -> &mut SearchStackEntry {
        &mut self.stack[self.ply]
    }

    pub const fn ss_at(&self, offset: usize) -> &SearchStackEntry {
        &self.stack[self.ply - offset]
    }

    pub const fn ss_at_mut(&mut self, offset: usize) -> &mut SearchStackEntry {
        &mut self.stack[self.ply - offset]
    }
}

impl Thread {
    pub fn update_tables(&mut self, best: Move, depth: usize, board: &Board, quiets: Vec<Move>, captures: Vec<Move>) {
        let (bonus, malus) = hist_delta(depth);
        self.caphist.update(board, best, &captures, bonus, malus);

        if best.flag().is_quiet() {
            self.history.update(board.stm, best, &quiets, bonus, malus);

            for i in 0..CONT_NUM {
                if let Some((p, tgt)) = self.get_previous_entry(1 + i) {
                    self.conthists[i].update(best, p, tgt, &quiets, bonus, malus);
                }
            }
        }
    }

    fn get_previous_entry(&self, rollback: usize) -> Option<(CPiece, Square)> {
        if self.ply >= rollback && self.stack[self.ply - rollback].move_made.is_some() {
            let se = &self.stack[self.ply - rollback];
            Some((se.moved.unwrap(), se.move_made.unwrap().tgt()))
        } else {
            None
        }
    }

    pub fn score_cap_hist(&self, m: Move, b: &Board) -> i32 {
        self.caphist.get_bonus(b, m)
    }

    pub fn assign_history_scores(&self, c: Color, moves: &[Move], scores: &mut [i32]) {
        for i in 0..moves.len() {
            scores[i] = self.history.get_bonus(c, moves[i]);
        }

        for i in 0..CONT_NUM {
            if let Some((piece, tgt)) = self.get_previous_entry(1 + i) {
                for j in 0..moves.len() {
                    scores[j] += self.conthists[i].get_bonus(moves[j], piece, tgt);
                }
            }
        }
    }
}
