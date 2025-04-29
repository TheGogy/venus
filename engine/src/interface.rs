use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};

use chess::movegen::perft::perft;

use crate::{position::pos::Pos, threading::threadpool::ThreadPool, timeman::time_control::TimeControl};

#[cfg(feature = "tune")]
use crate::tunables::params::tunables;

/// Engine struct.
/// This contains the thread pool,
/// the current position, and everything that stays constant between moves.
pub struct Engine {
    pub pos: Pos,
    pub pool: ThreadPool,
}

/// Engine interface.
/// This is how to communicate with the engine.
pub struct EngineInterface {
    stop: Arc<AtomicBool>,
    tx: mpsc::Sender<EngineCommand>,
}

/// Engine command.
/// List of all commands that the engine can be given.
pub enum EngineCommand {
    NewGame,
    SetOpt(String, String),
    Position(Box<Pos>),
    Go(TimeControl),
    Perft(usize),
    Stop,
    Eval,
}

/// Setup engine in new thread.
impl Default for EngineInterface {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let pool_stop = stop.clone();

        thread::spawn(move || Engine::run(rx, pool_stop));

        Self { stop, tx }
    }
}

impl EngineInterface {
    pub fn handle_command(&self, command: EngineCommand) {
        match command {
            EngineCommand::Stop => self.stop.store(true, Ordering::Relaxed),
            cmd => self.tx.send(cmd).unwrap(),
        }
    }
}

impl Engine {
    /// Run the engine.
    fn run(rx: mpsc::Receiver<EngineCommand>, stop: Arc<AtomicBool>) {
        let mut controller = Self { pos: Pos::default(), pool: ThreadPool::new(stop) };

        for c in rx {
            controller.handle_command(c);
        }
    }

    /// Handle commands.
    #[rustfmt::skip]
    fn handle_command(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::NewGame       => self.handle_newgame(),
            EngineCommand::SetOpt(n, v)  => self.handle_setopt(n, v),
            EngineCommand::Position(pos) => self.pos = *pos,
            EngineCommand::Go(tc)        => self.handle_go(tc),
            EngineCommand::Perft(d)      => self.handle_perft(d),
            EngineCommand::Eval          => self.handle_eval(),
            _ => eprintln!("Unknown command!")
        }
    }
}

/// Command handlers
impl Engine {
    /// Handle newgame command.
    fn handle_newgame(&mut self) {
        self.pos = Pos::default();
        self.pool.reset();
    }

    /// Handle go command.
    fn handle_go(&mut self, tc: TimeControl) {
        let bestmove = self.pool.go(&mut self.pos, tc);
        println!("bestmove {bestmove}");
    }

    /// Handle perft command.
    fn handle_perft(&mut self, d: usize) {
        let start = Instant::now();
        let total = perft::<true, true>(&mut self.pos.board.clone(), d);
        let duration = start.elapsed();

        let perf = total as u128 / duration.as_micros();
        println!("{:=^1$}", " Perft results ", 25);
        println!("  nodes: {total}");
        println!("  time:  {duration:?}");
        println!("  perf:  {perf} Mnps");
        println!("{:=^1$}", " <> ", 25);
    }

    /// Handle eval command.
    fn handle_eval(&self) {
        println!("{}", self.pos.evaluate());
    }

    /// Handle setopt command.
    fn handle_setopt(&mut self, n: String, v: String) {
        match &n[..] {
            "Threads" => {
                if let Ok(size) = v.parse::<usize>() {
                    if size > 0 {
                        self.pool.resize(size);
                    }
                }
            }

            #[cfg(feature = "tune")]
            _ => {
                if let Err(e) = tunables::set_tunable(&n, &v) {
                    eprintln!("Unsupported option: {n} ({e})");
                }
            }

            #[cfg(not(feature = "tune"))]
            _ => eprintln!("Unsupported option: {n}!"),
        }
    }
}
