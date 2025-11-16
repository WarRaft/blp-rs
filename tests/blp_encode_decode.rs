use blp::any_image::{AnyImage, EncodeOptions, EncodeMipOptions};
use std::fs;

#[test]
fn test_png_to_blp_and_extract() -> Result<(), Box<dyn std::error::Error>> {
    // Load PNG image
    let png_path = "preview/logo.png";
    let png_data = fs::read(png_path)?;
    let img = AnyImage::from_buffer(&png_data)?;
    
    println!("Loaded PNG: {}x{}", img.dimensions().0, img.dimensions().1);
    
    // Encode to BLP with mipmaps
    let blp_data = img.encode(&EncodeOptions::Blp {
        quality: 85,
        mip_options: Some(EncodeMipOptions {
            mip_count: None,      // Generate all mipmaps
            min_size: Some(4),    // Down to 4x4
            specific_mips: None,
            quantize_colors: None,
            quantize_dither: false,
        }),
        raw: None,
    })?;
    
    // Save BLP file
    let blp_output = "target/logo.blp";
    fs::write(blp_output, &blp_data)?;
    println!("Saved BLP: {} ({} bytes)", blp_output, blp_data.len());
    
    // Load the BLP back
    let blp_img = AnyImage::from_buffer(&blp_data)?;
    
    // Get number of mipmaps from frames
    let num_mips = blp_img.frames.len();
    println!("BLP has {} mipmap levels", num_mips);
    
    // Extract raw JPEG data for each mipmap
    fs::create_dir_all("target/raw_jpegs")?;
    for i in 0..num_mips {
        if let Ok(raw_jpeg) = blp_img.encode(&EncodeOptions::Blp {
            quality: 85,
            mip_options: None,
            raw: Some(i),
        }) {
            let raw_path = format!("target/raw_jpegs/logo_mip{}.jpg", i);
            fs::write(&raw_path, &raw_jpeg)?;
            println!("  Saved raw JPEG mip {}: {} ({} bytes)", 
                     i, raw_path, raw_jpeg.len());
        }
    }
    
    // Extract decoded mipmaps
    let decoded_mips = blp_img.decode_frames()?;
    
    // Save decoded mipmaps as PNG
    fs::create_dir_all("target/decoded_mips")?;
    for (i, mip_img) in decoded_mips.iter().enumerate() {
        let png_path = format!("target/decoded_mips/logo_mip{}.png", i);
        mip_img.save(&png_path)?;
        println!("  Saved decoded mip {}: {} ({}x{})", 
                 i, png_path, mip_img.width(), mip_img.height());
    }
    
    // Verify we can decode the main image
    let decoded = blp_img.into_dynamic()?;
    println!("Successfully decoded BLP to {}x{} image", 
             decoded.width(), decoded.height());
    
    Ok(())
}
