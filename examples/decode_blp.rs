use blp::blp_image_decoder::BlpImageDecoder;
use image::{ImageDecoder, RgbaImage};
use std::{env, fs::read, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    let inp = args
        .get(1)
        .expect("Usage: decode_blp <in.blp> [out.png]");
    let out = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("out.png");

    let data = read(inp)?;

    // Create decoder and get dimensions
    let dec = BlpImageDecoder::new(&data);
    let (w, h) = dec.dimensions();
    if w == 0 || h == 0 {
        return Err("invalid image dimensions".into());
    }

    // Allocate RGBA buffer and decode
    let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
    dec.read_image(&mut buf)?;

    // Convert raw RGBA to RgbaImage and save as PNG
    let img: RgbaImage = RgbaImage::from_raw(w, h, buf).ok_or("failed to create ImageBuffer")?;
    img.save(Path::new(out))?;

    println!("Saved {} ({}x{})", out, w, h);
    Ok(())
}
