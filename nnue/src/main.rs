use clap::{Parser, error::Result};
use nnue::arch::{QuantNNUEData, RawNNUEData};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(version, about)]
#[command(name = "ven-nnue")]
#[command(about = "Collection of NNUE utils.", long_about = None)]
#[command(arg_required_else_help = true)]
struct Args {
    /// Quantizes a (raw) model and dumps the output.
    #[arg(short, long = "quantize", value_names = ["INFILE", "OUTFILE"], num_args = 2)]
    quantize: Option<Vec<PathBuf>>,

    /// Permutes a (quantized) model and dumps the output.
    #[arg(short, long = "permute", value_names = ["INFILE", "OUTFILE"], num_args = 2)]
    permute: Option<Vec<PathBuf>>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(paths) = args.quantize {
        quantize(&paths[0], &paths[1]).unwrap_or_else(|e| e.exit())
    }

    Ok(())
}

pub fn quantize(infile: &PathBuf, outfile: &PathBuf) -> Result<()> {
    println!("[INFO] Quantizing {} -> {}", infile.display(), outfile.display());
    let raw_data = RawNNUEData::load_from_file(infile)?;
    let quant_data = raw_data.quantize();
    quant_data.write_to_file(outfile)?;
    println!("[INFO] Data has been written to '{}'.", outfile.display());

    Ok(())
}

pub fn permute(infile: &PathBuf, outfile: &PathBuf) -> Result<()> {
    println!("[INFO] Permuting {} -> {}", infile.display(), outfile.display());
    let quant_data = QuantNNUEData::load_from_file(infile)?;
    let perm_data = quant_data.permute();
    perm_data.write_to_file(outfile)?;
    println!("[INFO] Data has been written to '{}'.", outfile.display());

    Ok(())
}
