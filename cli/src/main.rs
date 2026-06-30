use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::uci::UCIReader;
use engine::bench::run_bench;
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

fn main() -> Result<()> {
    #[cfg(not(feature = "embed"))]
    println!("WARNING: engine does not have eval network. If you want to build the engine, make sure to build with the 'embed' feature.");

    let args = Args::parse();

    match args.command {
        Some(Command::Bench { epd }) => run_bench(epd),

        #[cfg(feature = "tune")]
        Some(Command::Spsa) => {
            println!("{}", tunables::spsa_output_txt());
            Ok(())
        }

        None => {
            UCIReader::default().run()?;
            Ok(())
        }
    }
}
