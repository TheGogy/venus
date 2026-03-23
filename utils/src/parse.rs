use std::fs::File;
use std::io::{BufRead, BufReader, Error};
use std::path::PathBuf;

/// Read a file into a list of lines, ignoring any lines that start with "#".
/// # Errors
///     Errors when file cannot be read.
pub fn parse_file_ignore_hash(path: PathBuf) -> Result<Vec<String>, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let lines = reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            if line.starts_with('#') { None } else { Some(line) }
        })
        .collect();

    Ok(lines)
}
