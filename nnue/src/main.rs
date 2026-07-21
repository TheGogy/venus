#![warn(clippy::all, clippy::pedantic)]

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, error::Result};
use nnue::{
    arch::{QuantNNUEData, RawNNUEData},
    preprocess::load_write::LoadWrite,
};
use utils::memory::Align64;

#[derive(Parser, Debug)]
#[command(name = "ven-nnue")]
#[command(version, about = "Collection of NNUE utils.")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Quantizes a (raw) model and dumps the output
    Quantize { infile: PathBuf, outfile: PathBuf },

    /// Permutes a (quantized) model and dumps the output
    Permute { infile: PathBuf, outfile: PathBuf },

    /// Get stats about the NNUE
    Stats { infile: PathBuf },
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Command::Quantize { infile, outfile } => quantize(&infile, &outfile),
        Command::Permute { infile, outfile } => permute(&infile, &outfile),
        Command::Stats { infile } => stats(&infile),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn quantize(infile: &Path, outfile: &Path) -> Result<()> {
    let raw_data = RawNNUEData::load_from_file(infile)?;
    println!("[INFO] Quantizing {} -> {}", infile.display(), outfile.display());
    let quant_data = raw_data.quantize();
    quant_data.write_to_file(outfile)?;
    println!("[INFO] Data has been written to '{}'.", outfile.display());

    Ok(())
}

fn permute(infile: &Path, outfile: &Path) -> Result<()> {
    let quant_data = QuantNNUEData::load_from_file(infile)?;
    println!("[INFO] Permuting {} -> {}", infile.display(), outfile.display());
    let perm_data = quant_data.permute();
    perm_data.write_to_file(outfile)?;
    println!("[INFO] Data has been written to '{}'.", outfile.display());

    Ok(())
}

fn print_stats_1d<T>(name: &str, vals: &[T])
where
    T: Copy + Into<f64>,
{
    #[allow(clippy::cast_precision_loss)]
    let len: f64 = vals.len() as f64;

    let mut sum = 0.0f64;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    for &v in vals {
        let x: f64 = v.into();
        sum += x;
        min = min.min(x);
        max = max.max(x);
    }

    let mean = sum / len;

    let variance = vals
        .iter()
        .map(|&v| {
            let x: f64 = v.into();
            (x - mean).powi(2)
        })
        .sum::<f64>()
        / len;

    let std = variance.sqrt();

    println!("{name:20} || min={min:>10.5} || max={max:>10.5} || mean={mean:>10.5} || std={std:>10.5}");
}

fn print_stats_2d<T, const N: usize>(name: &str, vals: &[Align64<[T; N]>])
where
    T: Copy + Into<f64>,
{
    println!("--- {name} ---");

    for (i, b) in vals.iter().enumerate() {
        print_stats_1d(format!("bkt {i} ==>").as_str(), &b.0);
    }
}

fn stats(file: &Path) -> Result<()> {
    let quant_data = QuantNNUEData::load_from_file(file)?;
    let nn = quant_data.prepare_nnue();
    println!("--- ft biases ---");
    print_stats_1d("ft biases", &nn.ftb.0);
    print_stats_2d("l1 weights", &nn.l1w);
    print_stats_2d("l1 biases", &nn.l1b);
    print_stats_2d("l2 weights", &nn.l2w);
    print_stats_2d("l2 biases", &nn.l2b);
    print_stats_2d("l3 weights", &nn.l3w);
    println!("--- l3 biases ---");
    for (b, w) in nn.l3b.iter().enumerate() {
        println!("bkt {b} ==> {w:>10.5}");
    }

    Ok(())
}
