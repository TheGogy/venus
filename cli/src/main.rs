use clap::{Parser, Subcommand};
use cli::uci::UCIReader;
use engine::interface::Engine;

#[cfg(feature = "tune")]
use engine::tunables::params::tunables;

#[derive(Parser, Debug)]
#[command(name = "Venus")]
#[command(version, about = "A strong NNUE chess engine.")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Runs a benchmark against a number of set test positions
    Bench,

    /// Outputs a list of the SPSA parameters for openbench
    #[cfg(feature = "tune")]
    Spsa,
}

fn main() {
    #[cfg(not(feature = "embed"))]
    println!("WARNING: engine does not have eval network. If you want to build the engine, make sure to build with the 'embed' feature.");

    let args = Args::parse();

    match args.command {
        Some(Command::Bench) => Engine::run_bench(),

        #[cfg(feature = "tune")]
        Some(Command::Spsa) => println!("{}", tunables::spsa_output_txt()),

        None => UCIReader::default().run(),
    }
}
