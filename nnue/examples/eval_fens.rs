//! Evaluate a list of FENs using a given network.

use std::{error::Error, io::BufRead, path::PathBuf};

use chess::types::board::Board;
use nnue::{arch::EmbedNNUEData, net::NNUE, preprocess::load_write::LoadWrite};

fn main() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: eval_fens <quantised.bin> < fens.txt"));
    let nn: &'static _ = Box::leak(EmbedNNUEData::load_from_file(&path)?.prepare_nnue());
    let mut nnue = NNUE::with_net(nn);

    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let fen = line.trim();
        if fen.is_empty() || fen.starts_with("#") {
            continue;
        }
        let b: Board = fen.parse()?;
        nnue.reset();
        nnue.update_all(&b);
        println!("{:.5}\t{fen}", nnue.evaluate_raw(&b));
    }

    Ok(())
}
