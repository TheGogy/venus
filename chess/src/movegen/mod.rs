pub mod enumerate;
pub mod make_move;
pub mod perft;
pub mod update;

// Movegen trait.
// Noisy: Only generate noisy moves (captures + queen promos).
// Quiet: Only generate quiet moves (non-captures).
// Allmv: Generate all moves.
pub trait MgType {
    const NOISY: bool;
    const QUIET: bool;
}

pub struct Noisy;
pub struct Quiet;
pub struct Allmv;

impl MgType for Noisy {
    const NOISY: bool = true;
    const QUIET: bool = false;
}

impl MgType for Quiet {
    const NOISY: bool = false;
    const QUIET: bool = true;
}

impl MgType for Allmv {
    const NOISY: bool = true;
    const QUIET: bool = true;
}
