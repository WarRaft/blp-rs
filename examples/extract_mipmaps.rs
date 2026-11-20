//! Extracts every mip of a BLP: decoded as PNG and, for JPEG textures,
//! the stored JPEG as is.
//!
//! Usage: cargo run --example extract_mipmaps input.blp output_dir

use blp::{AnyImage, EncodeOptions};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let [_, input, out_dir] = args.as_slice() else {
        eprintln!("Usage: {} <input.blp> <output_dir>", args[0]);
        std::process::exit(1);
    };

    let img = AnyImage::from_buffer(&fs::read(input)?)?;
    let (width, height) = img.dimensions();
    println!("{input}: {width}x{height}, {} mips", img.frames.len());

    fs::create_dir_all(format!("{out_dir}/decoded"))?;
    fs::create_dir_all(format!("{out_dir}/raw"))?;

    for (i, frame) in img.frames.iter().enumerate() {
        if frame.length == 0 {
            println!("  mip {i}: missing");
            continue;
        }
        let path = format!("{out_dir}/decoded/mip_{i:02}.png");
        img.decode_frame(i)?.save(&path)?;
        println!("  mip {i}: {}x{} -> {path}", frame.width, frame.height);

        if let Ok(jpeg) = img.encode(&EncodeOptions::Blp { quality: 0, mip_options: None, raw: Some(i) }) {
            let path = format!("{out_dir}/raw/mip_{i:02}.jpg");
            fs::write(&path, &jpeg)?;
            println!("  mip {i}: {} bytes of JPEG -> {path}", jpeg.len());
        }
    }
    Ok(())
}
