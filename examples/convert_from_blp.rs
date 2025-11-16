/// Example: Convert BLP to PNG/JPEG
///
/// Usage:
///   cargo run --example convert_from_blp input.blp output.png
///   cargo run --example convert_from_blp input.blp output.jpg 90

use blp::any_image::{AnyImage, EncodeOptions};
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <input.blp> <output> [jpeg_quality]", args[0]);
        eprintln!("  Supported outputs: .png, .jpg, .jpeg");
        eprintln!("  jpeg_quality: 0-100 (default: 90)");
        std::process::exit(1);
    }
    
    let input_path = &args[1];
    let output_path = &args[2];
    let jpeg_quality = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(90);
    
    println!("Converting BLP to image...");
    
    // Load BLP
    let blp_data = fs::read(input_path)?;
    let img = AnyImage::from_buffer(&blp_data)?;
    
    let (width, height) = img.dimensions();
    println!("  Input: {} ({}x{})", input_path, width, height);
    println!("  Mipmaps: {}", img.frames.len());
    
    // Determine output format from extension
    let ext = Path::new(output_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    let output_data = match ext.as_str() {
        "png" => img.encode(&EncodeOptions::Png { compression: Some(6) })?,
        "jpg" | "jpeg" => img.encode(&EncodeOptions::Jpeg { quality: jpeg_quality })?,
        _ => {
            eprintln!("Error: Unsupported output format '{}'", ext);
            eprintln!("Supported: png, jpg, jpeg");
            std::process::exit(1);
        }
    };
    
    // Save output
    fs::write(output_path, &output_data)?;
    
    println!("  Output: {} ({} bytes)", output_path, output_data.len());
    println!("✓ Conversion complete!");
    
    Ok(())
}
