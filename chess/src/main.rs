#![warn(clippy::all, clippy::pedantic)]

use chess::{helpers::see::bench_see, types::board::Board};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ven-chess")]
#[command(version, about = "Chess game implementation")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Runs a perft test up to the given depth
    Perft { depth: usize },

    /// Bench the SEE function this many times
    See { iters: usize },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Perft { depth } => println!("Total: {}", Board::default().perft::<true>(depth)),
        Command::See { iters } => bench_see(iters),
    }
}
