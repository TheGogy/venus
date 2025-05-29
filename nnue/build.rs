use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let evalfile_rel = env::var("EVALFILE").unwrap_or_else(|_| "data/nnue".to_string());

    let evalfile_path = if Path::new(&evalfile_rel).is_absolute() {
        PathBuf::from(&evalfile_rel)
    } else {
        manifest_dir.join(&evalfile_rel)
    };

    if !evalfile_path.exists() {
        panic!("EVALFILE path does not exist: {}", evalfile_path.display());
    }

    let canonical = evalfile_path.canonicalize().expect("Failed to canonicalize EVALFILE path");
    println!("cargo:rustc-env=NNUE_EVALFILE={}", canonical.display());
}
