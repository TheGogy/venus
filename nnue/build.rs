use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let evalfile_rel = env::var("EVALFILE").unwrap_or_else(|_| "data/nnue".to_string());

    // Make the path absolute relative to where cargo is run (the manifest dir)
    let evalfile_path = if Path::new(&evalfile_rel).is_absolute() {
        PathBuf::from(&evalfile_rel)
    } else {
        manifest_dir.join(&evalfile_rel)
    };

    if !evalfile_path.exists() {
        panic!("EVALFILE path does not exist: {}", evalfile_path.display());
    }

    // Canonicalize (absolute + resolve symlinks), then stringify
    let canonical = evalfile_path.canonicalize().expect("Failed to canonicalize EVALFILE path");

    // Output it as a string literal for include_bytes!
    println!("cargo:rustc-env=NNUE_EVALFILE={}", canonical.display());
}
