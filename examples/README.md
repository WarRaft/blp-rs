# Examples

This directory contains practical examples demonstrating how to use the blp-rs library.

## Running Examples

```bash
cargo run --example <example_name> -- [arguments]
```

## Available Examples

### 1. `convert_to_blp` - Convert any image to BLP

Convert PNG, JPEG, GIF, PSD, or any supported format to BLP with mipmap generation.

```bash
# Basic conversion
cargo run --example convert_to_blp input.png output.blp

# With quality setting (0-100)
cargo run --example convert_to_blp input.jpg output.blp 90

# With minimum mipmap size
cargo run --example convert_to_blp input.png output.blp 85 --min-size 16
```

**Features:**
- Automatic mipmap generation
- Quality control
- Minimum mipmap size control
- Power-of-two upscaling

---

### 2. `convert_from_blp` - Convert BLP to PNG/JPEG

Extract the main image from a BLP file and save as PNG or JPEG.

```bash
# Convert to PNG
cargo run --example convert_from_blp input.blp output.png

# Convert to JPEG with quality
cargo run --example convert_from_blp input.blp output.jpg 90
```

**Features:**
- Automatic format detection from extension
- JPEG quality control
- PNG compression control

---

### 3. `extract_mipmaps` - Extract all mipmap levels

Extract all mipmap levels from a BLP file, both as decoded PNG images and raw JPEG data.

```bash
cargo run --example extract_mipmaps input.blp output_directory/
```

**Output structure:**
```
output_directory/
├── decoded/
│   ├── mip_00.png  (1024x1024)
│   ├── mip_01.png  (512x512)
│   ├── mip_02.png  (256x256)
│   └── ...
└── raw/
    ├── mip_00.jpg  (raw JPEG data)
    ├── mip_01.jpg
    └── ...
```

**Features:**
- Extracts all mipmap levels
- Saves both decoded images and raw JPEG data
- Organized directory structure

---

### 4. `inspect_blp` - Inspect BLP metadata

View detailed metadata about a BLP file without decoding pixels.

```bash
cargo run --example inspect_blp input.blp
```

**Output includes:**
- BLP version (BLP0, BLP1, BLP2)
- Texture type (JPEG, PALETTE)
- Dimensions
- Alpha channel information
- Mipmap levels with sizes and offsets
- Header data size

**Features:**
- Fast metadata inspection (no pixel decoding)
- Detailed file structure information
- Useful for debugging and analysis

---

### 5. `convert_with_palette` - Palette-based compression (experimental)

Convert images using palette quantization for potentially better compression.

```bash
# With 256 colors
cargo run --example convert_with_palette input.png output.blp 256

# With 128 colors and dithering
cargo run --example convert_with_palette input.png output.blp 128 --dither
```

**Features:**
- Color quantization (1-256 colors)
- Optional dithering
- Useful for images with limited color palettes

**Note:** Palette-based compression is currently experimental. For production use, the default JPEG compression is recommended.

---

## Example Use Cases

### Quick format conversion
```bash
# PNG → BLP
cargo run --example convert_to_blp texture.png texture.blp

# BLP → PNG
cargo run --example convert_from_blp texture.blp texture.png
```

### Analyze BLP structure
```bash
# Inspect metadata
cargo run --example inspect_blp texture.blp

# Extract all mipmaps for inspection
cargo run --example extract_mipmaps texture.blp analysis/
```

### Quality optimization
```bash
# High quality for important textures
cargo run --example convert_to_blp hero.png hero.blp 95

# Lower quality for background textures
cargo run --example convert_to_blp background.png background.blp 75
```

### Controlled mipmap generation
```bash
# Generate mipmaps down to 16x16
cargo run --example convert_to_blp input.png output.blp 85 --min-size 16

# Palette quantization with dithering
cargo run --example convert_with_palette input.png output.blp 256 --dither
```

## Integration Examples

See the library documentation for programmatic usage:
- [`src/any_image.rs`](../src/any_image.rs) - Main API documentation
- [`tests/blp_encode_decode.rs`](../tests/blp_encode_decode.rs) - Comprehensive test example

## Need Help?

For more information:
- Read the [main README](../README.md)
- Check the [API documentation](https://docs.rs/blp)
- Browse the [test files](../tests/) for more usage patterns
