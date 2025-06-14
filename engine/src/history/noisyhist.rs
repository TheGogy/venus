use chess::types::{
    board::Board,
    moves::Move,
    piece::{CPiece, Piece},
    square::Square,
};

use super::{HistEntry, movebuffer::MoveBuffer};

/// [piecetype][captured][to]
#[derive(Clone, Debug)]
pub struct NoisyHist([[[HistEntry; Piece::NUM + 1]; CPiece::NUM]; Square::NUM]);

// TODO: add tunable history defaults.
impl Default for NoisyHist {
    fn default() -> Self {
        Self([[[HistEntry::default(); Piece::NUM + 1]; CPiece::NUM]; Square::NUM])
    }
}

pub const NOISY_MAX: i32 = 16384;

impl NoisyHist {
    fn idx(b: &Board, m: Move) -> (usize, usize, usize) {
        let cap = b.captured(m);
        let p_idx = if cap == CPiece::None { 6 } else { cap.pt().idx() };
        (m.dst().idx(), b.get_piece(m.src()).idx(), p_idx)
    }

    fn add_bonus(&mut self, b: &Board, m: Move, bonus: i16) {
        let i = Self::idx(b, m);
        self.0[i.0][i.1][i.2].gravity::<NOISY_MAX>(bonus);
    }

    pub fn get_bonus(&self, b: &Board, m: Move) -> i32 {
        let i = Self::idx(b, m);
        self.0[i.0][i.1][i.2].0 as i32
    }

    pub fn update(&mut self, b: &Board, best: Move, noisies: &MoveBuffer, bonus: i16, malus: i16) {
        if best.flag().is_cap() {
            self.add_bonus(b, best, bonus);
        }

        for m in noisies.iter() {
            self.add_bonus(b, *m, -malus);
        }
    }
}
