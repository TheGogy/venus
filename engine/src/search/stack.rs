use crate::threading::thread::Thread;

use chess::types::{eval::Eval, moves::Move, piece::CPiece};

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
