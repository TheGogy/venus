use chess::types::{
    board::Board,
    moves::Move,
    piece::{CPiece, Piece},
    square::Square,
};

use super::HistEntry;

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
    #[inline]
    fn idx(b: &Board, m: Move) -> (usize, usize, usize) {
        (m.dst().idx(), b.get_piece(m.src()).idx(), b.captured(m).pt().idx())
    }

    #[inline]
    fn add_bonus(&mut self, b: &Board, m: Move, bonus: i16) {
        let i = Self::idx(b, m);
        self.0[i.0][i.1][i.2].gravity::<NOISY_MAX>(bonus);
    }

    #[inline]
    pub fn get_bonus(&self, b: &Board, m: Move) -> i32 {
        let i = Self::idx(b, m);
        self.0[i.0][i.1][i.2].0 as i32
    }

    pub fn update(&mut self, b: &Board, best: Move, noisies: &Vec<Move>, bonus: i16, malus: i16) {
        if best.flag().is_cap() {
            self.add_bonus(b, best, bonus);
        }

        for m in noisies {
            self.add_bonus(b, *m, -malus);
        }
    }
}
