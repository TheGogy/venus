use std::env::args;

use cli::uci::UCIReader;
use engine::interface::Engine;

#[cfg(feature = "tune")]
use engine::tunables::params::tunables;

fn main() {
    match args().nth(1).as_deref() {
        // SPSA
        #[cfg(feature = "tune")]
        Some("spsa-json") => println!("{}", tunables::spsa_output_json()),
        #[cfg(feature = "tune")]
        Some("spsa-txt") => println!("{}", tunables::spsa_output_txt()),

        // Benchmarking
        Some("bench") => Engine::run_bench(),

        Some(&_) => println!("Unknown command!"),
        None => UCIReader::default().run(),
    }
}
