//! Permute a quantised network into the layout inference expects, and dump it directly.

use std::{io::Result, path::PathBuf};

use nnue::{arch::EmbedNNUEData, preprocess::load_write::LoadWrite};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1).map(PathBuf::from);
    let (infile, outfile) = match (args.next(), args.next()) {
        (Some(i), Some(o)) => (i, o),
        _ => panic!("usage: permute <quantised.bin> <permuted.bin>"),
    };

    EmbedNNUEData::load_from_file(&infile)?.prepare_nnue().write_to_file(&outfile)?;
    println!("{} -> {}", infile.display(), outfile.display());

    Ok(())
}
