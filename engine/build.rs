use std::{env, error::Error, fs::File, io::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    const LMR_BASE: f32 = 887.0 / 1024.0;
    const LMR_MULT: f32 = 2003.0 / 1024.0;

    let mut lmr_table = [[0; 64]; 64];

    for (depth, table) in lmr_table.iter_mut().enumerate().skip(1) {
        for (move_count, reduction) in table.iter_mut().enumerate().skip(1) {
            *reduction = (LMR_BASE + (depth as f32).ln() * (move_count as f32).ln() / LMR_MULT) as i16;
        }
    }

    let lmr = unsafe { std::slice::from_raw_parts::<u8>(lmr_table.as_ptr().cast::<u8>(), 64 * 64 * std::mem::size_of::<i16>()) };

    // Write to file in the output directory.
    let out_dir = env::var("OUT_DIR")?;
    let out_path = PathBuf::from(out_dir).join("lmr.bin");
    File::create(out_path)?.write_all(lmr)?;

    Ok(())
}
