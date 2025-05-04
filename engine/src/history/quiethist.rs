use chess::types::{board::Board, color::Color, moves::Move, square::Square};

use super::{HistEntry, movebuffer::MoveBuffer};

/// [color][from][to]
#[derive(Clone, Debug)]
pub struct QuietHist([[[HistEntry; Square::NUM]; Square::NUM]; Color::NUM]);

impl Default for QuietHist {
    fn default() -> Self {
        Self([[[HistEntry::default(); Square::NUM]; Square::NUM]; Color::NUM])
    }
}

pub const QUIET_MAX: i32 = 8192;

impl QuietHist {
    #[inline]
    const fn index(b: &Board, m: Move) -> (usize, usize, usize) {
        (b.stm.index(), m.src().index(), m.tgt().index())
    }

    #[inline]
    fn add_bonus(&mut self, b: &Board, m: Move, bonus: i16) {
        let i = Self::index(b, m);
        self.0[i.0][i.1][i.2].gravity::<QUIET_MAX>(bonus);
    }

    #[inline]
    pub fn get_bonus(&self, b: &Board, m: Move) -> i32 {
        let i = Self::index(b, m);
        self.0[i.0][i.1][i.2].0 as i32
    }

    pub fn update(&mut self, b: &Board, best: Move, quiets: &MoveBuffer, bonus: i16, malus: i16) {
        self.add_bonus(b, best, bonus);

        for m in quiets {
            self.add_bonus(b, *m, -malus);
        }
    }
}
