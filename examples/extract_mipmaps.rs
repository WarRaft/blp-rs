/// Example: Extract all mipmaps from BLP file
///
/// Usage:
///   cargo run --example extract_mipmaps input.blp output_dir

use blp::any_image::{AnyImage, EncodeOptions};
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <input.blp> <output_dir>", args[0]);
        std::process::exit(1);
    }
    
    let input_path = &args[1];
    let output_dir = &args[2];
    
    println!("Extracting mipmaps from {}...", input_path);
    
    // Load BLP
    let blp_data = fs::read(input_path)?;
    let img = AnyImage::from_buffer(&blp_data)?;
    
    let (width, height) = img.dimensions();
    println!("  BLP size: {}x{}", width, height);
    println!("  Mipmap levels: {}", img.frames.len());
    
    // Create output directories
    fs::create_dir_all(format!("{}/decoded", output_dir))?;
    fs::create_dir_all(format!("{}/raw", output_dir))?;
    
    // Extract decoded mipmaps using encode() with PNG
    for i in 0..img.frames.len() {
        let png_data = img.encode(&EncodeOptions::Png { compression: None })?;
        let png_path = format!("{}/decoded/mip_{:02}.png", output_dir, i);
        fs::write(&png_path, &png_data)?;
        
        // Load to get dimensions for logging
        let mip_img = image::load_from_memory(&png_data)?;
        println!("  ✓ Decoded mip {}: {}x{} -> {}", 
                 i, mip_img.width(), mip_img.height(), png_path);
    }
    
    // Extract raw JPEG data (if BLP uses JPEG compression)
    for i in 0..img.frames.len() {
        if let Ok(raw_jpeg) = img.encode(&EncodeOptions::Blp {
            quality: 85,
            mip_options: None,
            raw: Some(i),
        }) {
            let raw_path = format!("{}/raw/mip_{:02}.jpg", output_dir, i);
            fs::write(&raw_path, &raw_jpeg)?;
            println!("  ✓ Raw JPEG mip {}: {} bytes -> {}", 
                     i, raw_jpeg.len(), raw_path);
        }
    }
    
    println!("✓ Extraction complete! Output: {}", output_dir);
    
    Ok(())
}
