pub mod perftmp;
pub mod utils;

mod pick_move;
mod score_move;

use chess::{
    MAX_MOVES,
    types::{eval::Eval, moves::Move},
};

/// Move picker stages.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
#[repr(u8)]
#[allow(dead_code)] // Compiler does not like `.next()`.
pub enum MPStage {
    // PV search starts here.
    PvTT,
    PvNoisyGen,
    PvNoisyWin,
    PvQuietGen,
    PvQuietAll,
    PvNoisyLoss,
    PvEnd,

    // Qsearch starts here.
    QsTT,
    QsNoisyGen,
    QsNoisyAll,
    QsEnd,

    // Evasions start here.
    EvTT,
    EvGen,
    EvAll,
    EvEnd,
}

/// Search type.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
#[repr(u8)]
pub enum SearchType {
    Pv,
    Qs,
}

impl MPStage {
    /// Get the next move pick stage.
    pub fn next(self) -> Self {
        assert!(![MPStage::PvEnd, MPStage::QsEnd, MPStage::EvEnd].contains(&self));
        unsafe { std::mem::transmute(self as u8 + 1) }
    }
}

#[derive(Clone, Debug)]
pub struct MovePickerNew {
    stage: MPStage,
    searchtype: SearchType,

    tt_move: Move,

    // Constant for now, this is for when we implement probcut.
    see_threshold: Eval,

    skip_quiets: bool,

    mvs: [Move; MAX_MOVES],
    scs: [i32; MAX_MOVES],

    cur: usize,
    end: usize,

    noisy_loss_end: usize,
}

impl MovePickerNew {
    /// Construct a new move picker for the position.
    pub fn new(searchtype: SearchType, in_check: bool, tt_move: Move) -> Self {
        let mut stage = if in_check {
            MPStage::EvTT
        } else if searchtype == SearchType::Pv {
            MPStage::PvTT
        } else {
            MPStage::QsTT
        };

        let ttm = tt_move.is_valid_or(|| {
            stage = stage.next();
            Move::NONE
        });

        assert!(![MPStage::PvTT, MPStage::QsTT, MPStage::EvTT].contains(&stage) || !ttm.is_none());

        Self {
            stage,
            searchtype,
            tt_move: ttm,
            see_threshold: Eval::DRAW,
            skip_quiets: false,
            mvs: [Move::NONE; MAX_MOVES],
            scs: [0; MAX_MOVES],
            cur: 0,
            noisy_loss_end: MAX_MOVES - 1,
            end: 0,
        }
    }
}
