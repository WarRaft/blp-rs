use blp::Jpg;
use blp::jpg::JpgOptions;
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use image::codecs::png::PngEncoder;
use image::ImageEncoder;

#[test]
fn test_reencode_png_to_jpeg() {
    let w = 64u32; let h = 48u32;
    let img = ImageBuffer::from_pixel(w, h, Rgba([200u8, 100u8, 50u8, 255u8]));
    let dyn_img = image::DynamicImage::ImageRgba8(img);

    // encode PNG to buffer
    let mut png_buf: Vec<u8> = Vec::new();
    { let mut c = Cursor::new(&mut png_buf); let encoder = PngEncoder::new(&mut c); encoder.write_image(&dyn_img.to_rgba8(), w, h, image::ColorType::Rgba8.into()).unwrap(); }

    // Re-encode PNG bytes to JPEG
    let data = Jpg::encode(&png_buf, 0, JpgOptions::Reencode { quality: 85 }).expect("reencode to jpeg");
    assert!(data.len() > 0);
}

// BLP fixture is not included in repo; test is ignored by default
#[test]
#[ignore]
fn test_raw_blp_extraction() {
    let path = "tests/fixtures/sample.blp";
    let buf = std::fs::read(path).expect("Please add tests/fixtures/sample.blp to run BLP JPEG extraction tests");
    let data = Jpg::encode(&buf, 0, JpgOptions::Raw).expect("extract raw jpeg");
    assert!(data.len() > 0);
}
