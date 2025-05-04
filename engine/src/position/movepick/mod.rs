use super::pos::Pos;
use chess::{movegen::MGAllmv, types::move_list::MoveList};

mod pick_move;
mod score_move;
mod utils;

impl Pos {
    /// Initializes a movepicker from the current position.
    /// If there are no moves, returns None.
    pub fn init_movepicker<const QUIET: bool>(&self) -> Option<MovePicker<QUIET>> {
        let moves = self.board.gen_moves::<MGAllmv>();
        if moves.is_empty() { None } else { Some(MovePicker::new(moves)) }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
pub enum MPStage {
    NoisyScore,
    NoisyGood,
    QuietScore,
    QuietGood,
    NoisyBad,

    Finished,
}

#[derive(Clone, Debug)]
pub struct MovePicker<const QUIET: bool> {
    moves: MoveList,
    scores: [i32; MoveList::SIZE],

    stage: MPStage,

    idx_cur: usize,        // Current move.
    idx_good_quiet: usize, // Start of good quiets.
    idx_bad_noisy: usize,  // Start of bad noisies.
    idx_end: usize,        // End of move list.
}

impl<const QUIET: bool> MovePicker<QUIET> {
    /// Create a new move picker.
    pub const fn new(moves: MoveList) -> MovePicker<QUIET> {
        let end = moves.len();
        Self {
            moves,
            scores: [0; MoveList::SIZE],
            stage: MPStage::NoisyScore,
            idx_cur: 0,
            idx_good_quiet: 0,
            idx_bad_noisy: end,
            idx_end: end,
        }
    }
}
