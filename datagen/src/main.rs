#![warn(clippy::all, clippy::pedantic)]

use clap::{Parser, Subcommand};
use datagen::{
    datagen::{DataGenOpts, run_datagen},
    genfens::genfens,
};
use engine::bench::run_bench;

#[derive(Parser, Debug)]
#[command(name = "ven-datagen")]
#[command(version, about = "Datagen utilities for Venus.")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate training data through self play.
    Datagen(DataGenOpts),

    /// Openbench genfens command.
    Genfens { amount: usize, seed: u64 },

    /// Get the bench of the inner engine.
    Bench,
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Command::Datagen(opts) => run_datagen(opts),
        Command::Genfens { amount, seed } => {
            genfens(amount, seed);
            Ok(())
        }
        Command::Bench => run_bench(None),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
