pub mod enumerate;
pub mod make_move;
pub mod perft;
pub mod update;

pub trait MGType {
    const QUIET: bool;
    const NOISY: bool;
}

pub struct MGQuiet;
pub struct MGNoisy;
pub struct MGAllmv;

impl MGType for MGQuiet {
    const QUIET: bool = true;
    const NOISY: bool = false;
}

impl MGType for MGNoisy {
    const QUIET: bool = false;
    const NOISY: bool = true;
}

impl MGType for MGAllmv {
    const QUIET: bool = true;
    const NOISY: bool = true;
}
