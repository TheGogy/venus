use std::{
    io::{self, BufRead},
    str::SplitWhitespace,
};

use anyhow::{Result, anyhow};
#[cfg(feature = "tune")]
use engine::tunables::params::tunables;
use engine::{
    VERSION,
    bench::run_bench,
    interface::{EngineCommand, EngineInterface},
    position::Position,
    time_management::timecontrol::TimeControl,
};

pub const NAME: &str = "Venus";

/// Get authors formatted with commas.
fn authors() -> String {
    env!("CARGO_PKG_AUTHORS").replace(':', ", ")
}

pub const OPTS: &str = "
option name UCI_Chess960 type check default false
option name Threads type spin default 1 min 1 max 128
option name Hash type spin default 16 min 1 max 65536
option name Clear Hash type button";

#[cfg(feature = "syzygy")]
pub const SYZYGY_OPTS: &str = "
option name SyzygyPath type string default";

#[cfg(not(feature = "syzygy"))]
const SYZYGY_OPTS: &str = "";

#[derive(Default)]
pub struct UCIReader {
    interface: EngineInterface,
    // TODO: user friendly printer
}

impl UCIReader {
    /// Start UCI reader.
    pub fn run(&self) -> Result<()> {
        println!("{NAME} v{VERSION}-{} by {}", nnue::ARCH, authors());

        let stdin = io::stdin().lock();
        for line in stdin.lines() {
            let line = line?;
            match self.parse_command(&line) {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(e) => eprintln!("{e}"),
            }
        }

        Ok(())
    }

    /// Parse a UCI command. Returns true if the engine should quit.
    #[rustfmt::skip]
    fn parse_command(&self, s: &str) -> Result<bool> {
        let mut tokens = s.split_whitespace();

        match tokens.next() {
            Some(cmd) => match cmd {
                "quit"           => { self.interface.handle_command(EngineCommand::Stop); return Ok(true); }
                "isready"        => println!("readyok"),
                "bench"          => run_bench(None)?,
                "uci"            => self.cmd_uci(),
                "ucinewgame"     => self.interface.handle_command(EngineCommand::NewGame),
                "stop"           => self.interface.handle_command(EngineCommand::Stop),
                "eval"           => self.interface.handle_command(EngineCommand::Eval),
                "print" | "p"    => self.interface.handle_command(EngineCommand::Print),
                "perft"          => self.cmd_perft(&mut tokens)?,
                "perftmp"        => self.cmd_perftmp(&mut tokens)?,
                "go"             => self.cmd_go(&mut tokens)?,
                "position" | "b" => self.cmd_position(&mut tokens)?,
                "setoption"      => self.cmd_setoption(&mut tokens)?,
                "move" | "m"     => self.cmd_move(&mut tokens)?,
                "undo" | "u"     => self.interface.handle_command(EngineCommand::Undo),
                _ => return Err(anyhow!("Unknown command!"))
            },
            None => return Err(anyhow!("Empty command!")),
        }

        Ok(false)
    }
}

/// Parse a depth value from tokens.
fn parse_depth(tokens: &mut SplitWhitespace) -> Result<usize> {
    let depth: usize = tokens.next().ok_or_else(|| anyhow!("No depth value!"))?.parse().map_err(|_| anyhow!("Invalid depth value!"))?;

    if depth == 0 {
        return Err(anyhow!("Invalid depth value!"));
    }

    Ok(depth)
}

/// Commands.
impl UCIReader {
    /// uci command.
    pub fn cmd_uci(&self) {
        println!("id name {NAME}-{VERSION}");
        println!("id author {}", authors());
        println!("{OPTS}{SYZYGY_OPTS}");

        #[cfg(feature = "tune")]
        println!("{}", tunables::spsa_output_opts());

        println!("uciok");
    }

    /// perft command.
    pub fn cmd_perft(&self, tokens: &mut SplitWhitespace) -> Result<()> {
        let depth = parse_depth(tokens)?;
        self.interface.handle_command(EngineCommand::Perft(depth));
        Ok(())
    }

    /// perftmp command.
    pub fn cmd_perftmp(&self, tokens: &mut SplitWhitespace) -> Result<()> {
        let depth = parse_depth(tokens)?;
        self.interface.handle_command(EngineCommand::PerftMp(depth));
        Ok(())
    }

    /// go command.
    pub fn cmd_go(&self, tokens: &mut SplitWhitespace) -> Result<()> {
        let tc: TimeControl = tokens.collect::<Vec<&str>>().join(" ").parse().map_err(anyhow::Error::msg)?;
        self.interface.handle_command(EngineCommand::Go(tc));
        Ok(())
    }

    /// position command.
    pub fn cmd_position(&self, tokens: &mut SplitWhitespace) -> Result<()> {
        let pos: Position = tokens.collect::<Vec<&str>>().join(" ").parse().map_err(anyhow::Error::msg)?;
        self.interface.handle_command(EngineCommand::Position(Box::new(pos)));
        Ok(())
    }

    /// setoption command.
    pub fn cmd_setoption(&self, tokens: &mut SplitWhitespace) -> Result<()> {
        if tokens.next() != Some("name") {
            return Err(anyhow!("Invalid option command!"));
        }
        let name = tokens.next().ok_or_else(|| anyhow!("No option name!"))?.to_owned();

        if tokens.next() != Some("value") {
            return Err(anyhow!("Invalid option command!"));
        }
        let value = tokens.next().ok_or_else(|| anyhow!("No option value!"))?.to_owned();

        self.interface.handle_command(EngineCommand::SetOpt(name, value));
        Ok(())
    }

    /// move command.
    pub fn cmd_move(&self, tokens: &mut SplitWhitespace) -> Result<()> {
        let m = tokens.collect::<Vec<&str>>().join(" ");
        self.interface.handle_command(EngineCommand::Move(m));
        Ok(())
    }
}
