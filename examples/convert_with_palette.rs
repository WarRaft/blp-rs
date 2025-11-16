/// Example: Convert image to BLP with palette quantization
///
/// Usage:
///   cargo run --example convert_with_palette input.png output.blp 256
///   cargo run --example convert_with_palette input.png output.blp 128 --dither

use blp::any_image::{AnyImage, EncodeOptions, EncodeMipOptions};
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 4 {
        eprintln!("Usage: {} <input> <output.blp> <colors> [--dither]", args[0]);
        eprintln!("  colors: 1-256 (number of colors in palette)");
        eprintln!("  --dither: enable dithering for better quality");
        std::process::exit(1);
    }
    
    let input_path = &args[1];
    let output_path = &args[2];
    let colors: u8 = args[3].parse()
        .expect("Colors must be a number between 1 and 256");
    
    if colors == 0 {
        eprintln!("Error: colors must be at least 1");
        std::process::exit(1);
    }
    
    let dither = args.contains(&"--dither".to_string());
    
    println!("Converting {} to BLP with palette...", input_path);
    
    // Load input image
    let input_data = fs::read(input_path)?;
    let img = AnyImage::from_buffer(&input_data)?;
    
    let (width, height) = img.dimensions();
    println!("  Input: {}x{}", width, height);
    println!("  Palette colors: {}", colors);
    println!("  Dithering: {}", if dither { "enabled" } else { "disabled" });
    
    // Encode to BLP with palette quantization
    let blp_data = img.encode(&EncodeOptions::Blp {
        quality: 85,  // Quality still used for fallback JPEG compression
        mip_options: Some(EncodeMipOptions {
            mip_count: None,
            min_size: Some(4),
            specific_mips: None,
            quantize_colors: Some(colors),
            quantize_dither: dither,
        }),
        raw: None,
    })?;
    
    // Save BLP file
    fs::write(output_path, &blp_data)?;
    
    println!("  Output: {} ({} bytes)", output_path, blp_data.len());
    println!("✓ Conversion complete!");
    println!("\nNote: Palette-based compression is currently experimental.");
    println!("For production use, consider using JPEG compression (default).");
    
    Ok(())
}
