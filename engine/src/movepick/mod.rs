pub mod move_list;
pub mod perftmp;

mod pick_move;
mod score_move;

use chess::types::{eval::Eval, moves::Move};
use move_list::MoveList;

// ------------------------------------------------------------------------------------------------
//
// The movepicker sorts moves from what is probably the best move to what is probably the worst
// move. This allows us to have more cuts in alpha-beta pruning.
//
// For PV search, moves are stored as follows:
//
//  N = Winning noisy move.
//  n = Losing noisy move.
//  Q = Quiet move.
// +-----------+---------------+---------------------------------------------------+---------+
// | N N N N N | Q Q Q Q Q Q Q |                                                   | n n n n |
// +-----------+---------------+---------------------------------------------------+---------+
// ^           ^               ^                                                   ^
// cur         left            left (after enumerating quiets)                     right
//
// 1. We enumerate noisy moves. Winning ones are placed starting from the left, losing ones are placed
//    starting from the right.
// 2. We go through the winning noisy moves, up to end (as shown above).
// 3. We enumerate quiet moves, and put them all on the left.
// 4. We go through all quiet moves, up to end (as shown above, after enumerating quiets).
// 5. We go through the bad noisy moves, starting from the end of the list and working toward the
//    middle.
//
// For all other search types, we go through each move sequentially as they are generated.
//
// ------------------------------------------------------------------------------------------------

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
pub struct MovePicker {
    pub skip_quiets: bool,
    pub stage: MPStage,

    searchtype: SearchType,

    tt_move: Move,

    // Constant for now, this is for when we implement probcut.
    see_threshold: Eval,

    move_list: MoveList,
}

impl MovePicker {
    /// Construct a new move picker for the position.
    pub fn new(searchtype: SearchType, in_check: bool, tt_move: Move) -> Self {
        let mut stage = if in_check {
            MPStage::EvTT
        } else if searchtype == SearchType::Pv {
            MPStage::PvTT
        } else {
            MPStage::QsTT
        };

        let tt_move = tt_move.is_some_or(|| {
            stage = stage.next();
            Move::NONE
        });

        Self { stage, searchtype, tt_move, see_threshold: Eval::DRAW, skip_quiets: false, move_list: MoveList::default() }
    }
}

#[cfg(test)]
mod tests {
    use chess::types::{moves::MoveFlag, square::Square};

    use super::*;

    #[test]
    fn test_movepick_construction() {
        let mp = MovePicker::new(SearchType::Pv, false, Move::new(Square::E2, Square::E4, MoveFlag::Normal));
        assert_eq!(mp.stage, MPStage::PvTT);

        let mp = MovePicker::new(SearchType::Pv, false, Move::NONE);
        assert_eq!(mp.stage, MPStage::PvNoisyGen);

        let mp = MovePicker::new(SearchType::Pv, true, Move::NONE);
        assert_eq!(mp.stage, MPStage::EvGen);

        let mp = MovePicker::new(SearchType::Qs, false, Move::NONE);
        assert_eq!(mp.stage, MPStage::QsNoisyGen);
    }
}
