use chess::types::{moves::Move, piece::CPiece, square::Square};

use super::HistEntry;

#[derive(Clone, Debug)]
pub struct ContHist(Box<[[[[HistEntry; Square::NUM]; Square::NUM]; Square::NUM]; CPiece::NUM]>);

impl Default for ContHist {
    fn default() -> Self {
        Self(utils::box_array())
    }
}

pub const CONT_MAX: i32 = 16384;
pub const CONT_NUM: usize = 6;

impl ContHist {
    #[inline]
    fn index(m: Move, p: CPiece, s: Square) -> (usize, usize, usize, usize) {
        (p.index(), s.index(), m.src().index(), m.tgt().index())
    }

    #[inline]
    fn add_bonus(&mut self, m: Move, p: CPiece, s: Square, bonus: i16) {
        let i = Self::index(m, p, s);
        self.0[i.0][i.1][i.2][i.3].gravity::<CONT_MAX>(bonus);
    }

    #[inline]
    pub fn get_bonus(&self, m: Move, p: CPiece, s: Square) -> i32 {
        let i = Self::index(m, p, s);
        self.0[i.0][i.1][i.2][i.3].0 as i32
    }

    pub fn update(&mut self, best: Move, p: CPiece, tgt: Square, quiets: &Vec<Move>, bonus: i16, malus: i16) {
        self.add_bonus(best, p, tgt, bonus);

        for m in quiets {
            self.add_bonus(*m, p, tgt, -malus);
        }
    }
}
