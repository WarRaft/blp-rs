# blp-rs

[![Crates.io](https://img.shields.io/crates/v/blp.svg)](https://crates.io/crates/blp)
[![Documentation](https://docs.rs/blp/badge.svg)](https://docs.rs/blp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A pure Rust library for working with Blizzard's BLP texture format used in Warcraft III and World of Warcraft.

**Key Features:**
- 🚀 **Pure Rust** - No C dependencies, cross-platform (Windows, macOS, Linux)
- 🔄 **Complete Format Support** - BLP0, BLP1, BLP2 with JPEG and palette compression
- 🎨 **Universal Image Handling** - Works with BLP, PNG, JPEG, GIF, PSD, and more
- 🖼️ **Mipmap Control** - Precise control over mipmap generation and extraction
- ⚡ **High Performance** - Fast encoding/decoding using TurboJPEG
- 🎯 **Simple API** - Easy to use with comprehensive documentation

Part of the [WarRaft toolkit](https://github.com/WarRaft) for Warcraft III modding.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
blp = "1.0"
```

## Quick Start

### Convert Image to BLP

```rust
use blp::any_image::{AnyImage, EncodeOptions, EncodeMipOptions};

// Load any image format (PNG, JPEG, GIF, PSD, etc.)
let img_data = std::fs::read("input.png")?;
let img = AnyImage::from_buffer(&img_data)?;

// Convert to BLP with mipmaps
let blp_data = img.encode(&EncodeOptions::Blp {
    quality: 85,
    mip_options: Some(EncodeMipOptions {
        min_size: Some(4),  // Generate mipmaps down to 4x4
        ..Default::default()
    }),
    raw: None,
})?;

std::fs::write("output.blp", &blp_data)?;
```

### Convert BLP to PNG

```rust
use blp::any_image::{AnyImage, EncodeOptions};

let blp_data = std::fs::read("texture.blp")?;
let img = AnyImage::from_buffer(&blp_data)?;

let png_data = img.encode(&EncodeOptions::Png { 
    compression: Some(6) 
})?;

std::fs::write("output.png", &png_data)?;
```

### Extract Mipmaps

```rust
use blp::any_image::AnyImage;

let blp_data = std::fs::read("texture.blp")?;
let img = AnyImage::from_buffer(&blp_data)?;

// Get all mipmap levels as RgbaImage
let mipmaps = img.decode_frames()?;

for (i, mip) in mipmaps.iter().enumerate() {
    mip.save(format!("mip_{}.png", i))?;
}
```

## API Overview

### \`AnyImage\` - Universal Image Container

The main entry point for working with any image format:

```rust
// Load from buffer
let img = AnyImage::from_buffer(&data)?;

// Get dimensions
let (width, height) = img.dimensions();

// Convert to DynamicImage (from image crate)
let dyn_img = img.into_dynamic()?;

// Encode to different formats
let png = img.encode(&EncodeOptions::Png { compression: Some(6) })?;
let jpg = img.encode(&EncodeOptions::Jpeg { quality: 90 })?;
let blp = img.encode(&EncodeOptions::Blp { 
    quality: 85,
    mip_options: None,
    raw: None,
})?;

// Decode all frames/mipmaps
let frames = img.decode_frames()?;
```

### \`EncodeOptions\` - Output Format Configuration

Control how images are encoded:

```rust
// PNG with compression
EncodeOptions::Png { 
    compression: Some(6)  // 0-9, None for default
}

// JPEG with quality
EncodeOptions::Jpeg { 
    quality: 90  // 0-100
}

// BLP with full control
EncodeOptions::Blp {
    quality: 85,
    mip_options: Some(EncodeMipOptions { /* ... */ }),
    raw: None,  // Or Some(mip_index) to extract raw JPEG data
}
```

### \`EncodeMipOptions\` - Mipmap Generation Control

Fine-tune mipmap generation:

```rust
// Generate first 4 mipmaps only
EncodeMipOptions {
    mip_count: Some(4),
    ..Default::default()
}

// Generate until smallest dimension reaches 16px
EncodeMipOptions {
    min_size: Some(16),
    ..Default::default()
}

// Manual control over each mip level
EncodeMipOptions {
    specific_mips: Some(vec![true, true, false, true]),
    ..Default::default()
}

// Palette-based compression (experimental)
EncodeMipOptions {
    quantize_colors: Some(256),
    quantize_dither: true,
    ..Default::default()
}
```

### BLP-Specific Functions

Low-level BLP operations without full decoding:

```rust
use blp::blp;

// Inspect metadata without decoding pixels
let meta = blp::inspect_buf(&blp_data)?;
println!("Version: {:?}, Size: {}x{}", 
    meta.version, meta.width, meta.height);

// Get header data (JPEG header or palette)
if let Some(header) = blp::header_data(&blp_data) {
    println!("Header size: {} bytes", header.len());
}

// Get raw mipmap data
if let Some(raw) = blp::mip_raw(&blp_data, 0) {
    println!("Mip 0 size: {} bytes", raw.len());
}

// Quick dimension check
let (width, height) = blp::inspect_image_dimensions(&blp_data)?;
```

## Format Support

**Input Formats:**
- BLP (BLP0, BLP1, BLP2)
- PNG, JPEG, GIF, BMP, TIFF, WebP
- PSD (Photoshop documents)
- Any format supported by the [image](https://github.com/image-rs/image) crate

**Output Formats:**
- BLP (JPEG compression with automatic mipmap generation)
- PNG (with compression level control)
- JPEG (with quality control)

## Examples

The repository includes complete working examples in the [\`examples/\`](examples/) directory:

```bash
# Convert any image to BLP
cargo run --example convert_to_blp input.png output.blp 85

# Convert BLP to PNG
cargo run --example convert_from_blp input.blp output.png

# Extract all mipmaps from BLP
cargo run --example extract_mipmaps input.blp output_dir/

# Inspect BLP metadata
cargo run --example inspect_blp input.blp

# Palette-based conversion (experimental)
cargo run --example convert_with_palette input.png output.blp 256 --dither
```

See [\`examples/README.md\`](examples/README.md) for detailed documentation.

## Advanced Usage

### Raw JPEG Extraction from BLP

```rust
// Extract raw JPEG data for a specific mipmap level
let raw_jpeg = img.encode(&EncodeOptions::Blp {
    quality: 85,
    mip_options: None,
    raw: Some(0),  // Extract mip level 0
})?;

std::fs::write("mip_0.jpg", &raw_jpeg)?;
```

### Working with Individual Frames

```rust
// Access frame metadata
for (i, frame) in img.frames.iter().enumerate() {
    println!("Frame {}: {}x{}", i, frame.width, frame.height);
}

// Decode specific frames
let frames = img.decode_frames()?;
let first_frame = &frames[0];
```

### Custom Mipmap Generation

```rust
// Generate only specific mipmap levels
let mip_options = EncodeMipOptions {
    specific_mips: Some(vec![
        true,  // Mip 0: 1024x1024
        true,  // Mip 1: 512x512
        false, // Mip 2: skip
        true,  // Mip 3: 128x128
    ]),
    ..Default::default()
};

let blp_data = img.encode(&EncodeOptions::Blp {
    quality: 90,
    mip_options: Some(mip_options),
    raw: None,
})?;
```

## Performance

Built with performance in mind:

- **TurboJPEG** - Uses [turbojpeg](https://github.com/honzasp/rust-turbojpeg) for fast JPEG encoding/decoding
- **Efficient Memory Usage** - Lazy decoding, only loads what you need
- **Zero-Copy Operations** - Metadata inspection without pixel decoding

Typical performance on modern hardware:
- 1024x1024 BLP encoding with full mipmaps: ~100ms
- BLP → PNG conversion: ~50ms  
- Metadata inspection: <1ms (no pixel decoding)

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_png_to_blp_and_extract
```

## Documentation

- [API Documentation](https://docs.rs/blp)
- [Examples](examples/)
- [BLP Format Specification](https://github.com/WarRaft/BLP)

## Related Projects

- [WarRaft](https://github.com/WarRaft) - Warcraft III modding toolkit
- [JASS-Tree-sitter-Rust](https://github.com/WarRaft/JASS-Tree-sitter-Rust) - JASS language support
- [BLP Specification](https://github.com/WarRaft/BLP) - Detailed format documentation

## Contributing

Contributions are welcome! Feel free to:
- Report bugs
- Suggest features
- Submit pull requests
- Improve documentation

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <img src="https://raw.githubusercontent.com/WarRaft/blp-rs/refs/heads/main/preview/logo.png" alt="BLP Logo" width="256"/>
</p>
