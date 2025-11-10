use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

// Public, simple API exposed at crate root
use blp::{self, parse_blp_image, parse_blp_meta, BlpImage, TextureType};

/// Example: inspect a .blp file, dump metadata, extract JPEG mip tails (if present),
/// and save every decoded mip as a PNG.
///
/// Usage:
///   cargo run --example inspect_and_export_blp -- <input.blp> <out-dir>
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple CLI parsing: exactly two args required
    let mut args = env::args().skip(1);
    let input = args.next().ok_or("missing input path")?;
    let out = args.next().ok_or("missing output directory")?;

    let input = PathBuf::from(input);
    let out = PathBuf::from(out);
    fs::create_dir_all(&out)?;

    // Read file into memory
    let data = std::fs::read(&input)?;

    // 1) Inspect metadata (fast, no pixel decoding)
    let meta = parse_blp_meta(&data)?;

    let mut metadata_txt = String::new();
    metadata_txt.push_str(&format!("file: {}\n", input.display()));
    metadata_txt.push_str(&format!("version: {:?}\n", meta.version));
    metadata_txt.push_str(&format!("texture_type: {:?}\n", meta.texture_type));
    metadata_txt.push_str(&format!("size: {}x{}\n", meta.width, meta.height));
    metadata_txt.push_str(&format!("mips: {}\n", meta.mipmaps.len()));
    metadata_txt.push_str("mipmap list:\n");
    for m in &meta.mipmaps {
        metadata_txt.push_str(&format!("  - idx={} {}x{} off={} len={}\n", m.index, m.width, m.height, m.offset, m.length));
    }

    File::create(out.join("metadata.txt"))?.write_all(metadata_txt.as_bytes())?;

    // 2) Parse full BlpImage and decode all mip levels
    let mut img: BlpImage = parse_blp_image(&data)?;
    let mipmask = vec![true; img.mipmaps.len()];
    img.decode(&data, &mipmask)?; // decode every mip

    // 3) If the texture uses JPEG compression, extract raw JPEG bytes per mip
    if img.texture_type == TextureType::JPEG {
        for (i, m) in img.mipmaps.iter().enumerate() {
            if m.length == 0 { continue; }
            if let Some(tail) = img.mip_raw(&data, i) {
                let mut full = Vec::new();
                if let Some(hdr) = img.shared_jpeg_header(&data) {
                    full.extend_from_slice(hdr);
                }
                full.extend_from_slice(tail);
                let out_jpg = out.join(format!("mip_{:02}.jpg", i));
                std::fs::write(out_jpg, &full)?;
            }
        }
    }

    // 4) Save decoded mips to PNG files
    for (i, m) in img.mipmaps.iter().enumerate() {
        if let Some(ref ibuf) = m.image {
            let img_dyn = image::DynamicImage::ImageRgba8(ibuf.clone());
            let out_png = out.join(format!("mip_{:02}.png", i));
            // image::DynamicImage::save infers format from extension
            img_dyn.save(out_png)?;
        }
    }

    println!("Done — exported to {}", out.display());
    Ok(())
}
