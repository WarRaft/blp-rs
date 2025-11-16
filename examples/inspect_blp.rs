/// Example: Inspect BLP file metadata without decoding pixels
///
/// Usage:
///   cargo run --example inspect_blp input.blp

use blp::blp;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <input.blp>", args[0]);
        std::process::exit(1);
    }
    
    let input_path = &args[1];
    
    println!("Inspecting BLP file: {}", input_path);
    
    // Load BLP
    let blp_data = fs::read(input_path)?;
    
    // Inspect metadata without decoding pixels
    let meta = blp::inspect_buf(&blp_data)?;
    
    println!("\n=== BLP Metadata ===");
    println!("Version:       {:?}", meta.version);
    println!("Texture Type:  {:?}", meta.texture_type);
    println!("Compression:   {}", meta.compression);
    println!("Dimensions:    {}x{}", meta.width, meta.height);
    println!("Alpha Bits:    {}", meta.alpha_bits);
    println!("Alpha Type:    {}", meta.alpha_type);
    println!("Has Mips:      {}", meta.has_mips);
    println!("Has Mipmaps:   {}", meta.has_mipmaps);
    println!("Holes:         {}", meta.holes);
    println!("Extra:         {}", meta.extra);
    
    println!("\n=== Header Data ===");
    println!("Offset:        {}", meta.header_offset);
    println!("Length:        {} bytes", meta.header_length);
    
    println!("\n=== Mipmap Levels ({}) ===", meta.mipmaps.len());
    for mip in &meta.mipmaps {
        if mip.length > 0 {
            println!("  Mip {}: {}x{} @ offset {} ({} bytes)",
                     mip.index, mip.width, mip.height, mip.offset, mip.length);
        }
    }
    
    // Try to get header data
    if let Some(header) = blp::header_data(&blp_data) {
        println!("\n=== Header Bytes ===");
        match meta.texture_type {
            blp::TextureType::JPEG => {
                println!("JPEG Header: {} bytes", header.len());
            }
            blp::TextureType::PALETTE => {
                println!("Palette Data: {} bytes (256 RGBA entries)", header.len());
            }
        }
    }
    
    println!("\n✓ Inspection complete!");
    
    Ok(())
}
