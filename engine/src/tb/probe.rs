#![allow(clippy::cast_possible_truncation, clippy::upper_case_acronyms)]

use chess::{
    movegen::Allmv,
    types::{board::Board, castling::CastlingRights, color::Color, moves::Move, piece::Piece, square::Square},
};
use std::{ffi::CString, ptr, sync::atomic::AtomicU64};

#[cfg(feature = "syzygy")]
use crate::tb::binds::{
    TB_BLESSED_LOSS, TB_CURSED_WIN, TB_DRAW, TB_LARGEST, TB_LOSS, TB_PROMOTES_BISHOP, TB_PROMOTES_KNIGHT, TB_PROMOTES_QUEEN,
    TB_PROMOTES_ROOK, TB_RESULT_DTZ_MASK, TB_RESULT_DTZ_SHIFT, TB_RESULT_FAILED, TB_RESULT_FROM_MASK, TB_RESULT_FROM_SHIFT,
    TB_RESULT_PROMOTES_MASK, TB_RESULT_PROMOTES_SHIFT, TB_RESULT_TO_MASK, TB_RESULT_TO_SHIFT, TB_RESULT_WDL_MASK, TB_RESULT_WDL_SHIFT,
    TB_WIN, tb_probe_root, tb_probe_wdl,
};

/// Track the total TB hits.
pub static TB_HITS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WDL {
    Win,
    Loss,

    // CursedWin and BlessedLoss are winning/losing positions respectively with a 50mr cutoff.
    // We fold both of these into a draw.
    Draw,
}

#[derive(Clone, Copy, Debug)]
pub struct TbResult {
    pub wdl: WDL,
    pub dtz: u32,
    pub mov: Move,
}

#[derive(Clone, Debug, Default)]
pub struct SyzygyTB {
    pub is_active: bool,
    pub max_pcs: u32,
}

impl SyzygyTB {
    /// Load a syzygy tablebase from the given path.
    pub fn init(&mut self, path: &str) -> bool {
        #[cfg(feature = "syzygy")]
        unsafe {
            use crate::tb::binds::tb_init;

            let cpath = CString::new(path).unwrap();
            let tb = tb_init(cpath.as_ptr());

            // Failed to initialize tablebase - Possibly incorrect path?
            if !tb {
                return false;
            }

            // tb_init will also set TB_LARGEST.
            self.max_pcs = TB_LARGEST;
            self.is_active = true;

            true
        }

        #[cfg(not(feature = "syzygy"))]
        true
    }

    pub fn can_probe(&self, b: &Board) -> bool {
        self.is_active && b.occ().nbits() <= self.max_pcs && b.state.castling == CastlingRights::NONE
    }

    pub fn probe_wdl(&self, b: &Board) -> Option<WDL> {
        if !self.can_probe(b) || b.state.halfmoves > 0 {
            return None;
        }

        #[cfg(feature = "syzygy")]
        unsafe {
            let wdl = tb_probe_wdl(
                b.c_bb(Color::White).0,
                b.c_bb(Color::Black).0,
                b.p_bb(Piece::King).0,
                b.p_bb(Piece::Queen).0,
                b.p_bb(Piece::Rook).0,
                b.p_bb(Piece::Bishop).0,
                b.p_bb(Piece::Knight).0,
                b.p_bb(Piece::Pawn).0,
                0,
                0,
                if b.state.epsq == Square::Invalid { 0 } else { b.state.epsq as u32 },
                b.stm == Color::White,
            );

            match wdl {
                TB_WIN => Some(WDL::Win),
                TB_LOSS => Some(WDL::Loss),
                TB_DRAW | TB_CURSED_WIN | TB_BLESSED_LOSS => Some(WDL::Draw),
                _ => None,
            }
        }

        #[cfg(not(feature = "syzygy"))]
        None
    }

    pub fn probe_root(&self, b: &Board) -> Option<TbResult> {
        if !self.can_probe(b) {
            return None;
        }

        #[cfg(feature = "syzygy")]
        unsafe {
            let res = tb_probe_root(
                b.c_bb(Color::White).0,
                b.c_bb(Color::Black).0,
                b.p_bb(Piece::King).0,
                b.p_bb(Piece::Queen).0,
                b.p_bb(Piece::Rook).0,
                b.p_bb(Piece::Bishop).0,
                b.p_bb(Piece::Knight).0,
                b.p_bb(Piece::Pawn).0,
                b.state.halfmoves as u32,
                0,
                if b.state.epsq == Square::Invalid { 0 } else { b.state.epsq as u32 },
                b.stm == Color::White,
                ptr::null_mut(),
            );

            if res == TB_RESULT_FAILED {
                return None;
            }

            let wdl = match (res & TB_RESULT_WDL_MASK) >> TB_RESULT_WDL_SHIFT {
                TB_WIN => WDL::Win,
                TB_LOSS => WDL::Loss,
                _ => WDL::Draw,
            };

            let dtz = (res & TB_RESULT_DTZ_MASK) >> TB_RESULT_DTZ_SHIFT;

            let src = Square::from(((res & TB_RESULT_FROM_MASK) >> TB_RESULT_FROM_SHIFT) as u8);
            let dst = Square::from(((res & TB_RESULT_TO_MASK) >> TB_RESULT_TO_SHIFT) as u8);
            let promo = match (res & TB_RESULT_PROMOTES_MASK) >> TB_RESULT_PROMOTES_SHIFT {
                TB_PROMOTES_QUEEN => Piece::Queen,
                TB_PROMOTES_ROOK => Piece::Rook,
                TB_PROMOTES_BISHOP => Piece::Bishop,
                TB_PROMOTES_KNIGHT => Piece::Knight,
                _ => Piece::None,
            };

            let mut tb_res = None;
            b.enumerate_moves::<_, Allmv>(|m| {
                if m.src() == src && m.dst() == dst && (!m.flag().is_promo() || m.flag().get_promo() == promo) {
                    tb_res = Some(TbResult { wdl, dtz, mov: m });
                }
            });

            tb_res
        }

        #[cfg(not(feature = "syzygy"))]
        None
    }
}

#[cfg(test)]
#[cfg(feature = "syzygy")]
mod tests {
    use std::sync::LazyLock;

    use chess::types::board::Board;

    use crate::{
        position::Position,
        tb::probe::{SyzygyTB, WDL},
        threading::thread::Thread,
        tt::table::TT,
    };

    static TB: std::sync::LazyLock<SyzygyTB> = std::sync::LazyLock::new(|| {
        let mut tb = SyzygyTB::default();
        tb.init(std::env::var("SYZYGY_PATH").unwrap_or_else(|_| "/home/gogy/syzygy/".to_string()).as_str());
        tb
    });

    #[test]
    fn test_tb_wdl() {
        LazyLock::force(&TB);
        let win: Board = "4k3/8/1nb5/8/8/8/8/4K3 b - - 0 1".parse().unwrap();
        let draw: Board = "4k3/2r5/8/8/8/8/5B2/4K3 w - - 0 1".parse().unwrap();
        let loss: Board = "7r/6k1/8/4K3/8/8/8/8 w - - 0 1".parse().unwrap();
        assert_eq!(TB.probe_wdl(&win), Some(WDL::Win));
        assert_eq!(TB.probe_wdl(&draw), Some(WDL::Draw));
        assert_eq!(TB.probe_wdl(&loss), Some(WDL::Loss));
    }

    #[test]
    fn test_tb_full() {
        LazyLock::force(&TB);
        let b: Board = "4k3/8/1nb5/8/8/8/8/4K3 b - - 0 1".parse().unwrap();
        if let Some(res) = TB.probe_root(&b) {
            assert_eq!(res.wdl, WDL::Win);
            assert_eq!(res.dtz, 48);
            assert_eq!(res.mov.to_uci(&b.castlingmask), "b6c4");
        } else {
            panic!("Win probe failed!");
        }
    }

    #[test]
    fn test_tb_search() {
        LazyLock::force(&TB);
        let mut p: Position = "fen 5N1K/8/6k1/8/8/p7/6B1/8 b - - 0 1".parse().unwrap();
        let mut t = Thread::fixed_depth(2);
        let tt = TT::default();
        p.iterative_deepening::<true>(&mut t, &tt, &TB);

        assert_eq!(t.best_move().to_uci(&p.board.castlingmask), "g6f7");
    }
}
