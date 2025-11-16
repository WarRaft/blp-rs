use blp::any_image::{AnyImage, AnyImageData, EncodeOptions};
use image::{ImageBuffer, Rgba, GenericImageView};
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use image::ColorType;
use std::io::Cursor;

#[test]
fn test_gif_frames() {
    // create two simple frames and encode into GIF buffer
    let w = 16u32; let h = 16u32;
    let red = ImageBuffer::from_pixel(w, h, Rgba([255u8, 0u8, 0u8, 255u8]));
    let blue = ImageBuffer::from_pixel(w, h, Rgba([0u8, 0u8, 255u8, 255u8]));

    // Encode GIF into memory
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut buf);
        // Write frames with simple metadata
        let frame1 = image::Frame::from_parts(red.clone(), 0, 0, image::Delay::from_numer_denom_ms(100, 1));
        let frame2 = image::Frame::from_parts(blue.clone(), 0, 0, image::Delay::from_numer_denom_ms(100, 1));
        encoder.encode_frame(frame1).unwrap();
        encoder.encode_frame(frame2).unwrap();
    }

    // Parse using AnyImage
    let any = AnyImage::from_buffer(&buf).expect("AnyImage should parse GIF");
    // Data variant should be GIF and there must be two frames in metadata
    match &any.data {
        AnyImageData::Gif(_g) => {},
        other => panic!("Expected GIF variant, got {:?}", other),
    }
    assert_eq!(any.frames.len(), 2);

    // Decode frames using encode() with PNG format — should encode successfully
    for i in 0..2 {
        let png_data = any.encode(&EncodeOptions::Png { compression: None }).expect("Encode frame to PNG");
        assert!(!png_data.is_empty(), "Frame {} should encode to non-empty PNG", i);
    }
}

#[test]
fn test_single_image_frames_png() {
    // create a single PNG image in-memory and ensure AnyImage captures one frame
    let w = 32u32; let h = 16u32;
    let img = ImageBuffer::from_pixel(w, h, Rgba([18u8, 52u8, 86u8, 255u8]));
    let dyn_img = image::DynamicImage::ImageRgba8(img);

    // encode PNG to memory
    let mut png_buf: Vec<u8> = Vec::new();
    {
        let mut c = Cursor::new(&mut png_buf);
        let encoder = PngEncoder::new(&mut c);
        encoder.write_image(&dyn_img.to_rgba8(), w, h, ColorType::Rgba8.into()).expect("PNG encode");
    }

    let any = AnyImage::from_buffer(&png_buf).expect("AnyImage should parse PNG");
    // Data variant should be Image for PNG and AnyImage.frames contains exactly one entry
    match &any.data {
        AnyImageData::Image => {},
        other => panic!("Expected Image variant, got {:?}", other),
    }
    assert_eq!(any.frames.len(), 1);

    // Verify we can encode back to PNG
    let png_data = any.encode(&EncodeOptions::Png { compression: None }).expect("Encode to PNG");
    assert!(!png_data.is_empty());
    
    // Verify dimensions by loading the encoded PNG
    let reloaded = image::load_from_memory(&png_data).expect("Reload PNG");
    assert_eq!(reloaded.dimensions(), (w, h));
}
