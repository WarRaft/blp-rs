use blp::any_image::{AnyImage, AnyImageData, EncodeOptions};
use blp::image::GenericImageView;

#[test]
fn test_rgba_buffer_creation() {
    let width = 4u32;
    let height = 4u32;
    
    // Create RGBA buffer (red pixels)
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for i in 0..(width * height) as usize {
        rgba[i * 4] = 255;     // R
        rgba[i * 4 + 1] = 0;   // G
        rgba[i * 4 + 2] = 0;   // B
        rgba[i * 4 + 3] = 255; // A
    }
    
    // Create AnyImage from RGBA buffer
    let img = AnyImage::from_rgba(rgba, width, height).expect("Failed to create from RGBA");
    
    // Verify dimensions
    assert_eq!(img.dimensions(), (width, height));
    
    // Verify it's RgbaBuffer variant
    match &img.data {
        AnyImageData::RgbaBuffer { width: w, height: h } => {
            assert_eq!(*w, width);
            assert_eq!(*h, height);
        }
        _ => panic!("Expected RgbaBuffer variant"),
    }
    
    // Convert to dynamic image
    let dyn_img = img.into_dynamic().expect("Failed to convert to dynamic");
    assert_eq!(dyn_img.dimensions(), (width, height));
}

#[test]
fn test_rgba_to_png() {
    let width = 8u32;
    let height = 8u32;
    
    // Create gradient RGBA buffer
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            rgba[i] = (x * 32) as u8;     // R gradient
            rgba[i + 1] = (y * 32) as u8; // G gradient
            rgba[i + 2] = 128;            // B constant
            rgba[i + 3] = 255;            // A opaque
        }
    }
    
    let img = AnyImage::from_rgba(rgba, width, height).expect("Failed to create from RGBA");
    
    // Encode to PNG
    let png_data = img.encode(&EncodeOptions::Png { compression: None })
        .expect("Failed to encode to PNG");
    
    assert!(!png_data.is_empty());
    
    // Verify we can load it back
    let reloaded = AnyImage::from_buffer(&png_data).expect("Failed to reload PNG");
    assert_eq!(reloaded.dimensions(), (width, height));
}

#[test]
fn test_jpeg_detection() {
    // Load a JPEG file (using logo.png converted to JPEG for test)
    use std::fs;
    
    // First create a JPEG from PNG
    let png_data = fs::read("preview/logo.png").expect("Failed to read logo.png");
    let png_img = AnyImage::from_buffer(&png_data).expect("Failed to parse PNG");
    
    let jpeg_data = png_img.encode(&EncodeOptions::Jpeg { quality: 90 })
        .expect("Failed to encode to JPEG");
    
    // Now detect it as JPEG
    let jpg_img = AnyImage::from_buffer(&jpeg_data).expect("Failed to parse JPEG");
    
    // Verify it's detected as Jpg variant
    match &jpg_img.data {
        AnyImageData::Jpg(j) => {
            println!("JPEG detected: {}x{}", j.width, j.height);
            assert!(j.width > 0);
            assert!(j.height > 0);
        }
        _ => panic!("Expected Jpg variant, got {:?}", jpg_img.data),
    }
}
