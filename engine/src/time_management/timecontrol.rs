use std::{
    str::{FromStr, SplitWhitespace},
    time::Duration,
};

use chess::types::{Depth, color::Color};

/// Time controls supported by UCI.
/// This holds the max time that we have been given.
#[derive(Clone, Copy, Debug)]
pub enum TimeControl {
    Infinite,          // Infinite time.
    FixedDepth(Depth), // Keep searching up to this depth.
    FixedNodes(u64),   // Keep searching for this many nodes.
    FixedTime(u64),    // Keep searching for this long.
    Variable {
        wtime: u64,             // Max time for white.
        btime: u64,             // Max time for black.
        winc: Option<u64>,      // White increment.
        binc: Option<u64>,      // Black increment.
        movestogo: Option<u64>, // Number of moves until increment.
    },
}

/// Get a TC from a string in UCI time control format.
impl FromStr for TimeControl {
    type Err = &'static str;

    #[rustfmt::skip]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Default to infinite.
        if s.trim().is_empty() {
            return Ok(Self::Infinite);
        }

        let mut wtime:     Option<u64> = None;
        let mut btime:     Option<u64> = None;
        let mut winc:      Option<u64> = None;
        let mut binc:      Option<u64> = None;
        let mut movestogo: Option<u64> = None;

        let mut tokens = s.split_whitespace();
        while let Some(token) = tokens.next() {
            match token {
                // Fixed.
                "infinite" => return Ok(Self::Infinite),
                "depth"    => return Ok(Self::FixedDepth(parse(&mut tokens)?)),
                "nodes"    => return Ok(Self::FixedNodes(parse(&mut tokens)?)),
                "movetime" => return Ok(Self::FixedTime(parse(&mut tokens)?)),

                // Variable.
                "wtime"     => wtime = Some(parse(&mut tokens)?),
                "btime"     => btime = Some(parse(&mut tokens)?),
                "winc"      => winc  = Some(parse(&mut tokens)?),
                "binc"      => binc  = Some(parse(&mut tokens)?),
                "movestogo" => movestogo = Some(parse(&mut tokens)?),

                // Unimplemented.
                _ => return Err("Unknown token in time control!"),
            }
        }

        if let (Some(wtime), Some(btime)) = (wtime, btime) {
            Ok(Self::Variable { wtime, btime, winc, binc, movestogo })
        } else {
            Err("Invalid time control!")
        }
    }
}

/// Parse a value.
fn parse<T: FromStr>(tokens: &mut SplitWhitespace) -> Result<T, &'static str> {
    tokens.next().ok_or("Missing values for time control!")?.parse().or(Err("Unable to parse values for time control!"))
}

/// Get the optimal time values.
impl TimeControl {
    const OVERHEAD: u64 = 32;

    /// Get the optimal and maximum time from the time control.
    pub fn opt_max_time(self, stm: Color) -> (Duration, Duration) {
        match self {
            // These controls do not have maximum time.
            Self::Infinite | Self::FixedNodes(_) | Self::FixedDepth(_) => (Duration::ZERO, Duration::ZERO),

            // We are given this much time to make a move, so spend this much time.
            Self::FixedTime(t) => {
                let b = Duration::from_millis(t - Self::OVERHEAD.min(t));
                (b, b)
            }

            // Variable time control.
            Self::Variable { wtime, btime, winc, binc, movestogo } => {
                let (mut time, mut inc) = match stm {
                    Color::White => (wtime, winc.unwrap_or(0)),
                    Color::Black => (btime, binc.unwrap_or(0)),
                };

                time = time.saturating_sub(Self::OVERHEAD);
                if time < Self::OVERHEAD {
                    inc = 0;
                }

                let (opt, max) = match movestogo {
                    Some(mtg) => {
                        let scale = 0.7 / (mtg.min(50) as f64);
                        let eight = 0.8 * time as f64;

                        let opt = (scale * time as f64).min(eight);
                        let max = (5.0 * opt).min(eight);

                        (opt, max)
                    }

                    None => {
                        let total = ((time / 20) + (inc * 3 / 4)) as f64;

                        let opt = total * 0.6;
                        let max = (2.0 * total).min(time as f64);

                        (opt, max)
                    }
                };

                (Duration::from_millis(opt as u64), Duration::from_millis(max as u64))
            }
        }
    }
}
