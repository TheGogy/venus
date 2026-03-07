#[cfg(feature = "embed")]
use std::env;
#[cfg(feature = "embed")]
use std::path::{Path, PathBuf};

#[cfg(feature = "embed")]
const DEFAULT_EVALFILE: &str = "data/Cassini.bin";

fn main() {
    #[cfg(feature = "embed")]
    setup_evalfile();
}

#[cfg(feature = "embed")]
fn setup_evalfile() {
    let file = env::var("EVALFILE").unwrap_or_else(|_| DEFAULT_EVALFILE.to_string());
    let cwd = env::current_dir().expect("Failed to get current working directory");

    // Resolve relative to where the user ran make from.
    let abs_path = if Path::new(&file).is_absolute() { PathBuf::from(&file) } else { cwd.join(&file) };

    if !abs_path.exists() {
        panic!("EVALFILE path does not exist: {}", abs_path.display());
    }

    let canonical = abs_path.canonicalize().expect("Failed to canonicalize EVALFILE path");
    println!("cargo:rustc-env=NNUE_EVALFILE={}", canonical.display());
}
