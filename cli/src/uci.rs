use std::{
    io::{self, BufRead},
    str::SplitWhitespace,
};

use engine::{
    VERSION,
    interface::{EngineCommand, EngineInterface},
};

#[cfg(feature = "tune")]
use engine::tunables::params::tunables;

pub const NAME: &str = "Venus";
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// Get authors formatted with commas.
fn authors() -> String {
    AUTHORS.replace(':', ", ")
}

pub const OPTS: &str = "
option name UCI_Chess960 type check default false
option name Threads type spin default 1 min 1 max 128
option name Hash type spin default 16 min 1 max 65536
option name Clear Hash type button
";

#[derive(Default)]
pub struct UCIReader {
    interface: EngineInterface,
    // TODO: user friendly printer
}

impl UCIReader {
    /// Start UCI reader.
    pub fn run(&self) {
        println!("{NAME} v{VERSION} by {}", authors());
        #[cfg(feature = "tune")]
        println!("Tuning enabled.");

        let stdin = io::stdin().lock();
        for line in stdin.lines().map(Result::unwrap) {
            match self.parse_command(&line) {
                Ok(_) => {}
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
                "isready"     => println!("readyok"),
                "uci"         => self.cmd_uci(),
                "ucinewgame"  => self.interface.handle_command(EngineCommand::NewGame),
                "stop"        => self.interface.handle_command(EngineCommand::Stop),
                "eval"        => self.interface.handle_command(EngineCommand::Eval),
                "print" | "p" => self.interface.handle_command(EngineCommand::Print),
                "perft"       => return self.cmd_perft(&mut tokens),
                "go"          => return self.cmd_go(&mut tokens),
                "position"    => return self.cmd_position(&mut tokens),
                "setoption"   => return self.cmd_setoption(&mut tokens),

                "quit"        => std::process::exit(0),
                _ => return Err("Unknown command!")
            },
            None => return Err("Empty command!"),
        };

        Ok(())
    }
}

/// Commands.
impl UCIReader {
    /// uci command.
    pub fn cmd_uci(&self) {
        println!("id name {NAME}");
        println!("id author {}", authors());

        println!("{OPTS}");

        #[cfg(feature = "tune")]
        println!("{}", tunables::spsa_output_opts());

        println!("uciok");
    }

    /// perft command.
    pub fn cmd_perft(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        match tokens.next().ok_or("No depth value!")?.parse() {
            Ok(d) if d > 0 => self.interface.handle_command(EngineCommand::Perft(d)),
            _ => return Err("Invalid depth value!"),
        }
        Ok(())
    }

    /// go command.
    pub fn cmd_go(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let tc = tokens.collect::<Vec<&str>>().join(" ").parse()?;
        self.interface.handle_command(EngineCommand::Go(tc));
        Ok(())
    }

    /// position command.
    pub fn cmd_position(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let pos = tokens.collect::<Vec<&str>>().join(" ").parse()?;
        self.interface.handle_command(EngineCommand::Position(Box::new(pos)));
        Ok(())
    }

    /// setoption command.
    pub fn cmd_setoption(&self, tokens: &mut SplitWhitespace) -> Result<(), &'static str> {
        let name = match tokens.next() {
            Some("name") => tokens.next().ok_or("No option name!")?.to_owned(),
            _ => return Err("Invalid option command!"),
        };

        let value = match tokens.next() {
            Some("value") => tokens.next().ok_or("No option value!")?.to_owned(),
            _ => return Err("Invalid option command!"),
        };

        self.interface.handle_command(EngineCommand::SetOpt(name, value));
        Ok(())
    }
}
