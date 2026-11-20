//! Prints what a BLP's header says and checks that every mip decodes.
//! Given a folder, does so for every `.blp` in it and below.
//!
//! Usage: cargo run --example inspect_blp <file.blp | folder>

use blp::Blp;
use std::{env, fs, path::Path};

fn inspect(path: &Path, verbose: bool) -> Result<(), String> {
    let buf = fs::read(path).map_err(|e| e.to_string())?;
    let (blp, mips) = Blp::parse(&buf).map_err(|e| e.to_string())?;
    let encoding = blp
        .encoding()
        .map_err(|e| e.to_string())?;
    if verbose {
        println!("{}", path.display());
        println!("  {:?} {encoding}, {}x{}, alpha {} bits, extra {}, has mipmaps {}", blp.version, blp.width, blp.height, blp.alpha_bits, blp.extra, blp.has_mipmaps);
        println!("  shared header {} bytes at {}, {} unused bytes", blp.header.length, blp.header.offset, blp.holes);
    }
    for (i, mip) in mips.iter().enumerate() {
        match blp.decode(mip, &buf) {
            Ok(_) if verbose => println!("  mip {i}: {}x{}, {} bytes at {}", mip.width, mip.height, mip.length, mip.offset),
            Ok(_) => {}
            Err(e) if mip.length == 0 && verbose => println!("  mip {i}: {e}"),
            Err(_) if mip.length == 0 => {}
            Err(e) => return Err(format!("mip {i}: {e}")),
        }
    }
    Ok(())
}

fn walk(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("blp"))
        {
            found.push(path);
        }
    }
}

fn main() {
    let Some(target) = env::args().nth(1) else {
        eprintln!("Usage: inspect_blp <file.blp | folder>");
        std::process::exit(1);
    };
    let target = Path::new(&target);
    if !target.is_dir() {
        if let Err(e) = inspect(target, true) {
            eprintln!("  error: {e}");
            std::process::exit(1);
        }
        return;
    }
    let mut files = Vec::new();
    walk(target, &mut files);
    let failed: Vec<_> = files
        .iter()
        .filter_map(|f| inspect(f, false).err().map(|e| (f, e)))
        .collect();
    for (f, e) in &failed {
        println!("{}: {e}", f.display());
    }
    println!("{} files, {} failed", files.len(), failed.len());
}
