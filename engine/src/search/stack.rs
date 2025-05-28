use crate::threading::thread::Thread;

use chess::{
    MAX_DEPTH,
    types::{eval::Eval, moves::Move, piece::CPiece},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchStackEntry {
    pub mvp: CPiece,
    pub mov: Move,

    pub ply_from_null: usize,
    pub eval: Eval,
    pub ttpv: bool,
    pub excluded: Move,
}

impl Thread {
    pub const fn ss(&self) -> &SearchStackEntry {
        assert!(self.ply < MAX_DEPTH);
        &self.stack[self.ply]
    }

    pub const fn ss_mut(&mut self) -> &mut SearchStackEntry {
        assert!(self.ply < MAX_DEPTH);
        &mut self.stack[self.ply]
    }

    pub fn ss_at(&self, offset: usize) -> &SearchStackEntry {
        assert!(self.ply < MAX_DEPTH);
        assert!(offset <= self.ply);
        &self.stack[self.ply - offset]
    }

    pub const fn ss_at_mut(&mut self, offset: usize) -> &mut SearchStackEntry {
        assert!(self.ply < MAX_DEPTH);
        assert!(offset <= self.ply);
        &mut self.stack[self.ply - offset]
    }
}
