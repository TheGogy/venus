use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let cwd = env::current_dir().expect("Failed to get current working directory");
    let evalfile_rel = env::var("EVALFILE").unwrap_or_else(|_| "data/voyager-archv1-1536.bin".to_string());

    // Resolve relative to where the user ran make from.
    let evalfile_path = if Path::new(&evalfile_rel).is_absolute() {
        PathBuf::from(&evalfile_rel)
    } else {
        cwd.join(&evalfile_rel)
    };

    if !evalfile_path.exists() {
        panic!("EVALFILE path does not exist: {}", evalfile_path.display());
    }

    let canonical = evalfile_path.canonicalize().expect("Failed to canonicalize EVALFILE path");

    println!("cargo:rustc-env=NNUE_EVALFILE={}", canonical.display());
}
