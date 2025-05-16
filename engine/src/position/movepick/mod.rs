use chess::types::{move_list::MoveList, moves::Move};

mod pick_move;
mod score_move;
mod utils;

use super::pos::Pos;

impl Pos {
    pub fn init_movepicker<const QUIETS: bool>(&self, tt_move: Move) -> Option<MovePicker<QUIETS>> {
        let move_list = self.board.gen_moves::<QUIETS>();
        if move_list.is_empty() { None } else { Some(MovePicker::<QUIETS>::new(move_list, tt_move)) }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
pub enum Stage {
    TTMove,
    ScoreTacticals,
    GoodTacticals,
    ScoreQuiets,
    Quiets,
    BadTacticals,
    NoMoves,
}

#[derive(Clone, Debug)]
pub struct MovePicker<const QUIETS: bool> {
    moves: MoveList,
    scores: [i32; MoveList::SIZE],
    tt_move: Move,

    idx_cur: usize,
    idx_quiets: usize,
    idx_noisy_bad: usize,

    pub stage: Stage,
    pub skip_quiets: bool,
}

impl<const QUIETS: bool> MovePicker<QUIETS> {
    /// Initialize a new MovePicker for the given move list.
    pub fn new(move_list: MoveList, tt_move: Move) -> MovePicker<QUIETS> {
        let end = move_list.len();

        let stage = if tt_move.is_null() || (tt_move.flag().is_quiet() && !QUIETS) {
            Stage::ScoreTacticals
        } else {
            Stage::TTMove
        };

        MovePicker {
            moves: move_list,
            scores: [0; MoveList::SIZE],
            tt_move,

            idx_cur: 0,
            idx_quiets: 0,
            idx_noisy_bad: end,

            stage,
            skip_quiets: false,
        }
    }
}
