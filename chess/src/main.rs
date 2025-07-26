use std::env::args;

use chess::{helpers::see::bench_see, types::board::Board};

const HELP_MSG: &str = "Commands:
--help (-h)    Print this message.
--version (-v) Print the current version.

--perft        Run a perft test.
--see          Run a static eval test.";

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PERFT_DEPTH: usize = 7;
const SEE_ITERS: usize = 10000000;

fn main() {
    match args().nth(1).as_deref() {
        Some("--help") | Some("-h") => println!("{HELP_MSG}"),
        Some("--version") | Some("-v") => println!("{VERSION}"),

        Some("--perft") => _ = Board::default().perft::<false>(PERFT_DEPTH),
        Some("--see") => bench_see(SEE_ITERS),

        None | Some(&_) => println!("Unknown command! (run '--help')"),
    }
}
