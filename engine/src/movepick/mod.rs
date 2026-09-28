pub mod move_list;
pub mod perftmp;

mod pick_move;
mod score_move;

use chess::types::{eval::Eval, moves::Move};
use move_list::MoveList;

// The movepicker sorts moves from what is probably the best move to what is probably the worst
// move. This allows us to have more cuts in alpha-beta pruning.

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

    // Probcut starts here.
    PcTT,
    PcNoisyGen,
    PcNoisyAll,
    PcEnd,
}

/// Search type.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
#[repr(u8)]
pub enum SearchType {
    Pv,
    Qs,
    Pc,
}

impl MPStage {
    /// Get the next move pick stage.
    pub fn next(self) -> Self {
        assert!(!matches!(&self, MPStage::PvEnd | MPStage::QsEnd | MPStage::EvEnd | MPStage::PcEnd));
        unsafe { std::mem::transmute(self as u8 + 1) }
    }
}

#[derive(Clone, Debug)]
pub struct MovePicker {
    // Current search stage.
    pub stage: MPStage,
    // Whether or not we should skip quiet moves.
    pub skip_quiets: bool,
    // The move from the TT if it exists.
    tt_move: Move,
    // The SEE threshold (for Probcut).
    see_threshold: Eval,
    // List of moves and scores.
    move_list: MoveList,
}

impl MovePicker {
    /// Construct a new move picker for the position.
    pub fn new(searchtype: SearchType, in_check: bool, tt_move: Move, see_threshold: Eval) -> Self {
        let mut stage = if in_check {
            MPStage::EvTT
        } else {
            match searchtype {
                SearchType::Pv => MPStage::PvTT,
                SearchType::Qs => MPStage::QsTT,
                SearchType::Pc => MPStage::PcTT,
            }
        };

        let tt_move = tt_move.is_some_or(|| {
            stage = stage.next();
            Move::NONE
        });

        Self { stage, tt_move, see_threshold, skip_quiets: false, move_list: MoveList::default() }
    }
}
