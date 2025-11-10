use blp::BlpImageEncoder;
use image::ExtendedColorType;
use image::ImageEncoder;
use image::ImageReader;
use std::{env, fs::File, io::BufWriter, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    let inp = args
        .get(1)
        .expect("Usage: encode_blp <in.png> <out.blp>");
    let out = args
        .get(2)
        .expect("Usage: encode_blp <in.png> <out.blp>");

    // Load any image supported by `image` and convert to RGBA8
    let img = ImageReader::open(inp)?
        .decode()?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let buf = img.into_raw();

    // Write BLP using our encoder
    let f = File::create(Path::new(out))?;
    let enc = BlpImageEncoder::new(BufWriter::new(f));
    enc.write_image(&buf, w, h, ExtendedColorType::Rgba8)?;

    println!("Wrote {} ({}x{})", out, w, h);
    Ok(())
}
