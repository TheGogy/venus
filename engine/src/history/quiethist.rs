use chess::types::{color::Color, moves::Move, square::Square};

use super::{HistEntry, movebuffer::MoveBuffer};

/// [color][from][to]
#[derive(Clone, Debug)]
pub struct QuietHist([[[HistEntry; Square::NUM]; Square::NUM]; Color::NUM]);

// TODO: add tunable history defaults.
impl Default for QuietHist {
    fn default() -> Self {
        Self([[[HistEntry::default(); Square::NUM]; Square::NUM]; Color::NUM])
    }
}

pub const QUIET_MAX: i32 = 16384;

impl QuietHist {
    const fn idx(c: Color, m: Move) -> (usize, usize, usize) {
        (c.idx(), m.src().idx(), m.dst().idx())
    }

    fn add_bonus(&mut self, c: Color, m: Move, bonus: i16) {
        let i = Self::idx(c, m);
        self.0[i.0][i.1][i.2].gravity::<QUIET_MAX>(bonus);
    }

    pub fn get_bonus(&self, c: Color, m: Move) -> i32 {
        let i = Self::idx(c, m);
        self.0[i.0][i.1][i.2].0 as i32
    }

    pub fn update(&mut self, c: Color, best: Move, quiets: &MoveBuffer, bonus: i16, malus: i16) {
        self.add_bonus(c, best, bonus);

        for m in quiets.iter() {
            self.add_bonus(c, *m, -malus);
        }
    }
}
