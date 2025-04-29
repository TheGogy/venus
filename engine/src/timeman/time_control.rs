use std::str::{FromStr, SplitWhitespace};

/// Time control.
/// This represents the time constraints.
#[derive(PartialEq, Eq, PartialOrd, Clone, Copy, Debug)]
pub enum TimeControl {
    Infinite,
    FixedDepth(usize),
    FixedNodes(u64),
    FixedTime(u64),
    Variable(TimeSettings),
}

#[derive(PartialEq, Eq, PartialOrd, Clone, Copy, Debug, Default)]
pub struct TimeSettings {
    pub wtime: u64,
    pub btime: u64,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u64>,
}

fn parse<T: FromStr>(tokens: &mut SplitWhitespace) -> Result<T, &'static str> {
    tokens.next().ok_or("Missing values for time control!")?.parse().or(Err("Unable to parse values for time control!"))
}

/// Get time controls from a string according to UCI spec.
impl FromStr for TimeControl {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Ok(Self::Infinite);
        }

        let mut tokens = s.split_whitespace();

        // Handle simple time controls first
        if let Some(token) = tokens.next() {
            match token {
                "infinite" => return Ok(Self::Infinite),
                "depth" => return Ok(Self::FixedDepth(parse(&mut tokens)?)),
                "nodes" => return Ok(Self::FixedNodes(parse(&mut tokens)?)),
                "movetime" => return Ok(Self::FixedTime(parse(&mut tokens)?)),
                _ => (), // Pass on to TimeSettings
            }
        }

        // Parse variable time control
        let settings: TimeSettings = s.parse()?;
        Ok(TimeControl::Variable(settings))
    }
}

impl FromStr for TimeSettings {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut settings = TimeSettings::default();
        let mut tokens = s.split_whitespace();

        while let Some(token) = tokens.next() {
            match token {
                "wtime" => settings.wtime = parse::<i64>(&mut tokens)?.max(0) as u64,
                "btime" => settings.btime = parse::<i64>(&mut tokens)?.max(0) as u64,
                "winc" => settings.winc = Some(parse(&mut tokens)?),
                "binc" => settings.binc = Some(parse(&mut tokens)?),
                "movestogo" => settings.movestogo = Some(parse(&mut tokens)?),
                _ => (), // Ignore other tokens
            }
        }

        // Validate the settings
        if settings.wtime == 0 || settings.btime == 0 {
            return Err("Missing or invalid time control parameters");
        }

        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_control() {
        // Test simple time controls
        assert!(matches!("infinite".parse::<TimeControl>().unwrap(), TimeControl::Infinite));

        assert!(matches!("depth 6".parse::<TimeControl>().unwrap(), TimeControl::FixedDepth(6)));

        assert!(matches!("nodes 1000000".parse::<TimeControl>().unwrap(), TimeControl::FixedNodes(1000000)));

        assert!(matches!("movetime 5000".parse::<TimeControl>().unwrap(), TimeControl::FixedTime(5000)));

        // Test variable time control
        let tc = "wtime 300000 btime 300000 winc 2000 binc 2000 movestogo 40".parse::<TimeControl>().unwrap();

        if let TimeControl::Variable(settings) = tc {
            assert_eq!(settings.wtime, 300000);
            assert_eq!(settings.btime, 300000);
            assert_eq!(settings.winc, Some(2000));
            assert_eq!(settings.binc, Some(2000));
            assert_eq!(settings.movestogo, Some(40));
        } else {
            panic!("Expected Variable time control");
        }

        // Test with only partial parameters
        let tc = "wtime 300000 btime 300000".parse::<TimeControl>().unwrap();

        if let TimeControl::Variable(settings) = tc {
            assert_eq!(settings.wtime, 300000);
            assert_eq!(settings.btime, 300000);
            assert_eq!(settings.winc, None);
            assert_eq!(settings.binc, None);
            assert_eq!(settings.movestogo, None);
        } else {
            panic!("Expected Variable time control");
        }
    }
}
