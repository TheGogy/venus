use chess::types::{
    board::Board,
    moves::Move,
    piece::{CPiece, Piece},
    square::Square,
};

use super::{HistEntry, movebuffer::MoveBuffer};

/// [piecetype][captured][to]
#[derive(Clone, Debug)]
pub struct NoisyHist([[[HistEntry; Piece::NUM - 1]; CPiece::NUM]; Square::NUM]);

impl Default for NoisyHist {
    fn default() -> Self {
        Self([[[HistEntry::default(); Piece::NUM - 1]; CPiece::NUM]; Square::NUM])
    }
}

pub const NOISY_MAX: i32 = 16384;

impl NoisyHist {
    #[inline]
    fn index(b: &Board, m: Move) -> (usize, usize, usize) {
        (m.tgt().index(), b.get_piece(m.src()).index(), b.captured(m).pt().index())
    }

    #[inline]
    fn add_bonus(&mut self, b: &Board, m: Move, bonus: i16) {
        let i = Self::index(b, m);
        assert!(i.0 < Square::NUM);
        assert!(i.1 < CPiece::NUM);
        assert!(i.2 < Piece::NUM, "{} {:?} {:?}", m, m.flag(), b.captured(m).pt());
        self.0[i.0][i.1][i.2].gravity::<NOISY_MAX>(bonus);
    }

    #[inline]
    pub fn get_bonus(&self, b: &Board, m: Move) -> i32 {
        let i = Self::index(b, m);
        self.0[i.0][i.1][i.2].0 as i32
    }

    pub fn update(&mut self, b: &Board, best: Move, noisies: &MoveBuffer, bonus: i16, malus: i16) {
        if best.flag().is_cap() {
            self.add_bonus(b, best, bonus);
        }

        for m in noisies {
            self.add_bonus(b, *m, -malus);
        }
    }
}
