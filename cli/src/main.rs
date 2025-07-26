use std::env::args;

use cli::uci::UCIReader;
use engine::{VERSION, interface::Engine};

#[cfg(feature = "tune")]
use engine::tunables::params::tunables;

const HELP_MSG: &str = "Commands:
(no args)      Run the UCI interface.

--help (-h)    Print this message.
--version (-v) Print the current version.

bench          Run the benchmarking suite.";

#[cfg(feature = "tune")]
const TUNE_HELP_MSG: &str = "
spsa-json      Output the SPSA tuning parameters in json format.
spsa-txt       Output the SPSA tuning parameters in raw format.
";

#[cfg(not(feature = "tune"))]
const TUNE_HELP_MSG: &str = "";

fn main() {
    match args().nth(1).as_deref() {
        // CLI args
        Some("--help") | Some("-h") => println!("{HELP_MSG}{TUNE_HELP_MSG}"),
        Some("--version") | Some("-v") => println!("{VERSION}"),

        // SPSA
        #[cfg(feature = "tune")]
        Some("spsa-json") => println!("{}", tunables::spsa_output_json()),
        #[cfg(feature = "tune")]
        Some("spsa-txt") => println!("{}", tunables::spsa_output_txt()),

        // Benchmarking
        Some("bench") => Engine::run_bench(),

        Some(&_) => println!("Unknown command! (run '--help')"),
        None => UCIReader::default().run(),
    }
}
