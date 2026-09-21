use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};

#[cfg(feature = "tune")]
use crate::tunables::params::tunables;
use crate::{
    position::Position,
    tb::probe::SyzygyTB,
    threading::{thread::Thread, threadpool::ThreadPool},
    time_management::timecontrol::TimeControl,
    tt::table::TT,
};

/// Engine struct.
/// This contains the thread pool,
/// the current position, and everything that stays constant between moves.
pub struct Engine {
    pub pos: Position,
    pub pool: ThreadPool,
    pub tt: TT,
    pub tb: SyzygyTB,
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
    Position(Box<Position>),
    Go(TimeControl),
    Perft(usize),
    PerftMp(usize),
    Print,
    Stop,
    Eval,
    Move(String),
    Undo,
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
            cmd => self.tx.send(cmd).unwrap_or_else(|_| println!("Failed to send command!")),
        }
    }
}

impl Engine {
    /// Run the engine.
    fn run(rx: mpsc::Receiver<EngineCommand>, stop: Arc<AtomicBool>) {
        let mut controller = Self { pos: Position::default(), pool: ThreadPool::new(stop), tt: TT::default(), tb: SyzygyTB::default() };

        for c in rx {
            controller.handle_command(c);
        }
    }

    /// Handle commands.
    #[rustfmt::skip]
    fn handle_command(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::NewGame       => self.handle_newgame(),
            EngineCommand::SetOpt(n, v)  => self.handle_setopt(&n, &v),
            EngineCommand::Position(pos) => self.pos = *pos,
            EngineCommand::Go(tc)        => self.handle_go(tc),
            EngineCommand::Perft(d)      => self.handle_perft::<false>(d),
            EngineCommand::PerftMp(d)    => self.handle_perft::<true>(d),
            EngineCommand::Eval          => self.handle_eval(),
            EngineCommand::Move(m)       => self.handle_move(&m),
            EngineCommand::Undo          => self.handle_undo(),
            EngineCommand::Print         => println!("{}", self.pos.board),

            // Should have been handled already.
            EngineCommand::Stop          => unreachable!()
        }
    }
}

/// Command handlers
impl Engine {
    /// Handle newgame command.
    fn handle_newgame(&mut self) {
        self.pos.reset();
        self.pool.reset();
        self.tt.clear();
    }

    /// Handle go command.
    fn handle_go(&mut self, tc: TimeControl) {
        self.tt.increment_age();
        let bestmove = self.pool.go(&mut self.pos, tc, &self.tt, &self.tb);
        println!("bestmove {}", bestmove.to_uci(&self.pos.board.castlingmask));
    }

    /// Handle perft command.
    fn handle_perft<const MP: bool>(&mut self, depth: usize) {
        let start = Instant::now();
        let total = if MP { self.pos.perftmp::<true>(depth) } else { self.pos.board.perft::<true>(depth) };
        let duration = start.elapsed();

        let perf = total as u128 / duration.as_micros();
        println!("{:=^1$}", " Perft results ", 25);
        println!("  nodes: {total}");
        println!("  time:  {duration:?}");
        println!("  perf:  {perf} Mnps");
        println!("  mp?:   {MP}");
        println!("{:=^1$}", " <> ", 25);
    }

    /// Handle eval command.
    fn handle_eval(&mut self) {
        if self.pos.board.in_check() {
            println!("NOTE: In check - board will not be evaluated.");
        }

        println!("{}", self.pos.evaluate());
    }

    /// Handle setopt command.
    fn handle_setopt(&mut self, n: &str, v: &str) {
        match n {
            "Threads" => {
                if let Ok(size) = v.parse::<usize>()
                    && size > 0
                {
                    self.pool.resize(size - 1);
                }
            }

            "Hash" => {
                if let Ok(size) = v.parse::<usize>()
                    && size > 0
                {
                    self.tt.resize(size);
                }
            }

            "UCI_Chess960" => {
                if let Ok(val) = v.parse::<bool>() {
                    self.pos.board.castlingmask.frc = val;
                }
            }

            "Clear" => {
                if v == "Hash" {
                    self.tt.clear();
                }
            }

            "SyzygyPath" => {
                if self.tb.init(v) {
                    println!("info string found syzygy tb at {v} ({}-man)", self.tb.max_pcs);
                } else {
                    println!("info string could not find syzygy tb at {v}");
                }
            }

            #[cfg(feature = "tune")]
            _ => {
                if tunables::set_tunable(n, v).is_err() {
                    println!("Unsupported option: {n}!");
                }
            }

            #[cfg(not(feature = "tune"))]
            _ => eprintln!("Unsupported option: {n}!"),
        }
    }

    /// Handle move command.
    fn handle_move(&mut self, m: &str) {
        let Some(mv) = self.pos.board.find_move(m) else {
            println!("Move not found!");
            return;
        };
        self.pos.make_move(mv, &mut Thread::placeholder());
    }

    /// Handle undo command.
    fn handle_undo(&mut self) {
        self.pos.undo_move(&mut Thread::placeholder());
    }
}
