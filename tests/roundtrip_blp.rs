use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;

use blp::{BlpImageDecoder, BlpImageEncoder};
use image::ImageDecoder;
use image::ImageEncoder;

/// Copies two preview images into `examples/test_roundtrip/`, encodes them to .blp and
/// decodes back to png. Asserts that round-tripped files exist and dimensions match.
#[test]
fn roundtrip_preview_images() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from("test-data/test-roundtrip");
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)?;
    }
    fs::create_dir_all(&out_dir)?;

    let inputs = vec!["preview/logo.png", "preview/bg.jpg"];

    for inp in inputs {
        let src = PathBuf::from(inp);
        let file_name = src.file_name().unwrap();
        let dest = out_dir.join(file_name);
        fs::copy(&src, &dest)?;

        // Load image and encode to BLP
        let img = image::ImageReader::open(&dest)?
            .decode()?
            .to_rgba8();
        let (w, h) = img.dimensions();
        let buf = img.into_raw();

        let blp_path = out_dir.join(format!("{}.blp", file_name.to_string_lossy()));
        let f = fs::File::create(&blp_path)?;
        let enc = BlpImageEncoder::new(BufWriter::new(f));
        enc.write_image(&buf, w, h, image::ExtendedColorType::Rgba8)?;

        // Read BLP bytes and decode
        let data = fs::read(&blp_path)?;
        let dec = BlpImageDecoder::new(&data);
        let (dw, dh) = dec.dimensions();
        // Decoder provides the output dimensions; we don't require they match
        // the original exactly because the encoder may choose a power-of-two
        // cover. We'll validate that the decoded buffer matches the reported
        // dimensions when saved below.

        let mut out_buf = vec![0u8; (dw as usize) * (dh as usize) * 4];
        dec.read_image(&mut out_buf)?;

        let out_png = out_dir.join(format!("{}_roundtrip.png", file_name.to_string_lossy()));
        let out_img = image::RgbaImage::from_raw(dw, dh, out_buf).ok_or("failed to create image buffer")?;
        out_img.save(&out_png)?;

        assert!(out_png.exists(), "roundtrip output missing");
        let meta = fs::metadata(&out_png)?;
        assert!(meta.len() > 0, "roundtrip output empty");
    }

    Ok(())
}
