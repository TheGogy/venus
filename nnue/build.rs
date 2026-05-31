#[cfg(feature = "embed")]
use std::env;
#[cfg(feature = "embed")]
use std::path::PathBuf;

fn main() {
    #[cfg(feature = "embed")]
    setup_evalfile();
}

#[cfg(feature = "embed")]
fn setup_evalfile() {
    let path = PathBuf::from(env::var("EVALFILE").expect("Must define EVALFILE"));
    assert!(path.is_absolute(), "EVALFILE must be absolute");

    if !path.exists() {
        panic!("EVALFILE path does not exist: {}", path.display());
    }

    let canonical = path.canonicalize().expect("Failed to canonicalize EVALFILE path");
    println!("cargo:rerun-if-env-changed=EVALFILE");
    println!("cargo:rerun-if-changed={}", canonical.display());
}
