use chess::types::{moves::Move, piece::CPiece, square::Square};

use super::{HistEntry, movebuffer::MoveBuffer};

#[derive(Clone, Debug)]
pub struct ContHist(Box<[[[[HistEntry; Square::NUM]; Square::NUM]; Square::NUM]; CPiece::NUM]>);

// TODO: add tunable history defaults.
impl Default for ContHist {
    fn default() -> Self {
        Self(utils::box_array())
    }
}

pub const CONT_MAX: i32 = 16384;
pub const CONT_NUM: usize = 6;

pub type PieceTo = (CPiece, Square);

impl ContHist {
    fn idx(m: Move, pt: PieceTo) -> (usize, usize, usize, usize) {
        (pt.0.idx(), pt.1.idx(), m.src().idx(), m.dst().idx())
    }

    fn add_bonus(&mut self, m: Move, pt: PieceTo, bonus: i16) {
        let i = Self::idx(m, pt);
        self.0[i.0][i.1][i.2][i.3].gravity::<CONT_MAX>(bonus);
    }

    pub fn get_bonus(&self, m: Move, pt: PieceTo) -> i32 {
        let i = Self::idx(m, pt);
        self.0[i.0][i.1][i.2][i.3].0 as i32
    }

    pub fn update(&mut self, best: Move, pt: PieceTo, quiets: &MoveBuffer, bonus: i16, malus: i16) {
        self.add_bonus(best, pt, bonus);

        for m in quiets.iter() {
            self.add_bonus(*m, pt, -malus);
        }
    }
}
