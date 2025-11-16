/// Example: Convert any image format (PNG, JPEG, GIF, PSD, etc.) to BLP
///
/// Usage:
///   cargo run --example convert_to_blp input.png output.blp
///   cargo run --example convert_to_blp input.jpg output.blp 85
///   cargo run --example convert_to_blp input.psd output.blp 90 --min-size 16

use blp::any_image::{AnyImage, EncodeOptions, EncodeMipOptions};
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <input> <output.blp> [quality] [--min-size N]", args[0]);
        eprintln!("  quality: 0-100 (default: 85)");
        eprintln!("  --min-size: minimum mipmap dimension (default: generate all)");
        std::process::exit(1);
    }
    
    let input_path = &args[1];
    let output_path = &args[2];
    let quality = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(85);
    
    // Parse min-size option
    let min_size = args.iter().enumerate()
        .find(|(_, arg)| arg.as_str() == "--min-size")
        .and_then(|(i, _)| args.get(i + 1))
        .and_then(|s| s.parse().ok());
    
    println!("Converting {} to BLP...", input_path);
    
    // Load input image
    let input_data = fs::read(input_path)?;
    let img = AnyImage::from_buffer(&input_data)?;
    
    let (width, height) = img.dimensions();
    println!("  Input: {}x{}", width, height);
    
    // Encode to BLP with mipmaps
    let blp_data = img.encode(&EncodeOptions::Blp {
        quality,
        mip_options: Some(EncodeMipOptions {
            mip_count: None,
            min_size,
            specific_mips: None,
            quantize_colors: None,
            quantize_dither: false,
        }),
        raw: None,
    })?;
    
    // Save BLP file
    fs::write(output_path, &blp_data)?;
    
    println!("  Output: {} ({} bytes)", output_path, blp_data.len());
    println!("  Quality: {}", quality);
    if let Some(min) = min_size {
        println!("  Min mipmap size: {}x{}", min, min);
    }
    println!("✓ Conversion complete!");
    
    Ok(())
}
