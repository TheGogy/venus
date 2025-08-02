use chess::{
    defs::MAX_PLY,
    types::{eval::Eval, moves::Move},
};

use crate::history::conthist::PieceTo;

use super::thread::Thread;

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchStackEntry {
    pub pieceto: Option<PieceTo>,
    pub ply_from_null: usize,
    pub eval: Eval,
    pub excluded: Option<Move>,
    pub killer: Option<Move>,
}

impl Thread {
    /// Get the current search stack entry.
    pub const fn ss(&self) -> &SearchStackEntry {
        assert!(self.ply < MAX_PLY);
        &self.stack[self.ply]
    }

    /// Get the current search stack entry (mutable).
    pub const fn ss_mut(&mut self) -> &mut SearchStackEntry {
        assert!(self.ply < MAX_PLY);
        &mut self.stack[self.ply]
    }

    /// Get the search stack entry some offset from the top.
    pub const fn ss_at(&self, offset: usize) -> &SearchStackEntry {
        assert!(self.ply < MAX_PLY);
        assert!(offset <= self.ply);
        &self.stack[self.ply - offset]
    }

    /// Get the search stack entry some offset from the top (mutable).
    pub const fn ss_at_mut(&mut self, offset: usize) -> &mut SearchStackEntry {
        assert!(self.ply < MAX_PLY);
        assert!(offset <= self.ply);
        &mut self.stack[self.ply - offset]
    }

    /// Clear the next node.
    pub const fn prepare_next(&mut self) {
        self.stack[self.ply + 1].killer = None;
    }
}
