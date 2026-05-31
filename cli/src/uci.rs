use std::{
    io::{self, BufRead},
    str::SplitWhitespace,
};

#[cfg(feature = "tune")]
use engine::tunables::params::tunables;
use engine::{
    VERSION,
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
    pub fn run(&self) {
        println!("{NAME} v{VERSION}-{} by {}", nnue::ARCH, authors());

        let stdin = io::stdin().lock();
        for line in stdin.lines().map(Result::unwrap) {
            match self.parse_command(&line) {
                Ok(()) => {}
                Err(e) => eprintln!("{e}"),
            }
        }
    }

    /// Parse a UCI command.
    #[rustfmt::skip]
    fn parse_command(&self, s: &str) -> Result<(), &'static str> {
        let mut tokens = s.split_whitespace();

        match tokens.next() {
            Some(cmd) => match cmd {
                "isready"        => println!("readyok"),
                "uci"            => self.cmd_uci(),
                "ucinewgame"     => self.interface.handle_command(EngineCommand::NewGame),
                "stop"           => self.interface.handle_command(EngineCommand::Stop),
                "eval"           => self.interface.handle_command(EngineCommand::Eval),
                "print" | "p"    => self.interface.handle_command(EngineCommand::Print),
                "perft"          => return self.cmd_perft(&mut tokens),
                "perftmp"        => return self.cmd_perftmp(&mut tokens),
                "go"             => return self.cmd_go(&mut tokens),
                "position" | "b" => return self.cmd_position(&mut tokens),
                "setoption"      => return self.cmd_setoption(&mut tokens),
                "move" | "m"     => return self.cmd_move(&mut tokens),
                "undo" | "u"     => self.interface.handle_command(EngineCommand::Undo),
                _ => return Err("Unknown command!")
            },
            None => return Err("Empty command!"),
        }

        Ok(())
    }
}

/// Parse a depth value from tokens.
fn parse_depth(tokens: &mut SplitWhitespace) -> Result<usize, &'static str> {
    let depth: usize = tokens.next().ok_or("No depth value!")?.parse().map_err(|_| "Invalid depth value!")?;

    if depth == 0 {
        return Err("Invalid depth value!");
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
    pub fn cmd_perft(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let depth = parse_depth(tokens)?;
        self.interface.handle_command(EngineCommand::Perft(depth));
        Ok(())
    }

    /// perftmp command.
    pub fn cmd_perftmp(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let depth = parse_depth(tokens)?;
        self.interface.handle_command(EngineCommand::PerftMp(depth));
        Ok(())
    }

    /// go command.
    pub fn cmd_go(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let tc: TimeControl = tokens.collect::<Vec<&str>>().join(" ").parse()?;
        self.interface.handle_command(EngineCommand::Go(tc));
        Ok(())
    }

    /// position command.
    pub fn cmd_position(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let pos: Position = tokens.collect::<Vec<&str>>().join(" ").parse()?;
        self.interface.handle_command(EngineCommand::Position(Box::new(pos)));
        Ok(())
    }

    /// setoption command.
    pub fn cmd_setoption(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        if tokens.next() != Some("name") {
            return Err("Invalid option command!");
        }
        let name = tokens.next().ok_or("No option name!")?.to_owned();

        if tokens.next() != Some("value") {
            return Err("Invalid option command!");
        }
        let value = tokens.next().ok_or("No option value!")?.to_owned();

        self.interface.handle_command(EngineCommand::SetOpt(name, value));
        Ok(())
    }

    /// move command.
    pub fn cmd_move(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let m = tokens.collect::<Vec<&str>>().join(" ");
        self.interface.handle_command(EngineCommand::Move(m));
        Ok(())
    }
}
