use std::path::PathBuf;

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
    Bench { epd: Option<PathBuf> },

    /// Outputs a list of the SPSA parameters for openbench
    #[cfg(feature = "tune")]
    Spsa,
}

fn main() {
    #[cfg(not(feature = "embed"))]
    println!("WARNING: engine does not have eval network. If you want to build the engine, make sure to build with the 'embed' feature.");

    let args = Args::parse();

    let result = match args.command {
        Some(Command::Bench { epd }) => Engine::run_bench(epd),

        #[cfg(feature = "tune")]
        Some(Command::Spsa) => {
            println!("{}", tunables::spsa_output_txt());
            Ok(())
        }

        None => {
            UCIReader::default().run();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
