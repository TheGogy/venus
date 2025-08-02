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
    PvKiller,
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
    pub skip_quiets: bool,
    pub stage: MPStage,

    searchtype: SearchType,

    tt_move: Move,
    killer: Move,

    see_threshold: Eval,

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

        Self { stage, searchtype, tt_move, killer: Move::NONE, see_threshold, skip_quiets: false, move_list: MoveList::default() }
    }
}

#[cfg(test)]
mod tests {
    use chess::types::{moves::MoveFlag, square::Square};

    use super::*;

    #[test]
    fn test_movepick_construction() {
        let mp = MovePicker::new(SearchType::Pv, false, Move::new(Square::E2, Square::E4, MoveFlag::Normal), Eval::DRAW);
        assert_eq!(mp.stage, MPStage::PvTT);

        let mp = MovePicker::new(SearchType::Pv, false, Move::NONE, Eval::DRAW);
        assert_eq!(mp.stage, MPStage::PvNoisyGen);

        let mp = MovePicker::new(SearchType::Pv, true, Move::NONE, Eval::DRAW);
        assert_eq!(mp.stage, MPStage::EvGen);

        let mp = MovePicker::new(SearchType::Qs, false, Move::NONE, Eval::DRAW);
        assert_eq!(mp.stage, MPStage::QsNoisyGen);
    }
}
