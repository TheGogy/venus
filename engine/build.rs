use std::{env, error::Error, fs::File, io::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    write_lmr_table()?;

    #[cfg(feature = "syzygy")]
    setup_fathom()?;

    Ok(())
}

fn write_lmr_table() -> Result<(), Box<dyn Error>> {
    const LMR_BASE: f32 = 0.8662109;
    const LMR_MULT: f32 = 1.9560547;

    let mut lmr_table = [[0; 64]; 64];

    for (depth, table) in lmr_table.iter_mut().enumerate().skip(1) {
        for (move_count, reduction) in table.iter_mut().enumerate().skip(1) {
            *reduction = (LMR_BASE + (depth as f32).ln() * (move_count as f32).ln() / LMR_MULT) as i32;
        }
    }

    let lmr = unsafe { std::slice::from_raw_parts::<u8>(lmr_table.as_ptr().cast::<u8>(), 64 * 64 * std::mem::size_of::<i32>()) };

    // Write to file in the output directory.
    let out_dir = env::var("OUT_DIR")?;
    let out_path = PathBuf::from(out_dir).join("lmr.bin");
    File::create(out_path)?.write_all(lmr)?;

    Ok(())
}

#[cfg(feature = "syzygy")]
fn setup_fathom() -> Result<(), Box<dyn Error>> {
    // Write bindings.
    let binds = bindgen::Builder::default()
        .header("./external/Fathom/src/tbprobe.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .unwrap();

    binds.write_to_file("./src/tb/binds.rs").unwrap();

    // Compile Fathom.
    let cc = &mut cc::Build::new();
    cc.file("./external/Fathom/src/tbprobe.c");
    cc.include("./external/Fathom/src/");

    // The functions are safe! Their readme says so!
    cc.define("_CRT_SECURE_NO_WARNINGS", None);

    let target_cpu = std::env::var("TARGET_CPU").unwrap_or("native".to_string());
    cc.flag(format!("-march={}", target_cpu));
    cc.flag("-march=native");
    cc.flag("-w");
    cc.compile("fathom");

    Ok(())
}
