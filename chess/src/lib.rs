pub mod helpers;
pub mod macros;
pub mod movegen;
pub mod tables;
pub mod types;
pub mod utils;

// See https://chess.stackexchange.com/questions/4490/maximum-possible-movement-in-a-turn
pub const MAX_MOVES: usize = 220;

// The highest depth we will search to.
pub const MAX_PLY: usize = 127;

pub type Depth = i16;
