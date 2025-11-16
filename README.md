# blp-rs

[![Crates.io](https://img.shields.io/crates/v/blp.svg)](https://crates.io/crates/blp)
[![Documentation](https://docs.rs/blp/badge.svg)](https://docs.rs/blp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A pure Rust library and toolkit for working with Blizzard's BLP texture format used in Warcraft III and World of Warcraft.

**Features:**
- 🚀 Pure Rust - no C dependencies, works everywhere (Windows, macOS, Linux)
- 🔄 Full format support - BLP0, BLP1, BLP2 with JPEG and palette compression
- 🎨 Universal image handling - works with PNG, JPEG, GIF, PSD, and more
- 🖼️ Mipmap control - precise control over mipmap generation
- ⚡ High performance - efficient encoding and decoding
- 🎯 Simple API - easy to use, well-documented

Part of the [WarRaft toolkit](https://github.com/WarRaft) for Warcraft III modding.

Want to know how BLP works? Check out the [BLP Specification](https://github.com/WarRaft/BLP).

## Quick Start

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
blp = "1.0"
```

**Convert any image to BLP:**

```rust
use blp::any_image::{AnyImage, EncodeOptions, EncodeMipOptions};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load any image format (PNG, JPEG, GIF, PSD, etc.)
    let img_data = fs::read("input.png")?;
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
    
    fs::write("output.blp", &blp_data)?;
    Ok(())
}
```

**Convert BLP to PNG:**

```rust
use blp::any_image::{AnyImage, EncodeOptions};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let blp_data = fs::read("input.blp")?;
    let img = AnyImage::from_buffer(&blp_data)?;
    
    let png_data = img.encode(&EncodeOptions::Png { compression: Some(6) })?;
    fs::write("output.png", &png_data)?;
    Ok(())
}
```

**Extract all mipmaps:**

```rust
use blp::any_image::AnyImage;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let blp_data = fs::read("input.blp")?;
    let img = AnyImage::from_buffer(&blp_data)?;
    
    // Decode all mipmap levels
    let mipmaps = img.decode_frames()?;
    
    for (i, mip) in mipmaps.iter().enumerate() {
        mip.save(format!("mip_{}.png", i))?;
    }
    Ok(())
}
```

### Command Line Examples

The repository includes several runnable examples:

```bash
# Convert any image to BLP
cargo run --example convert_to_blp input.png output.blp 85

# Convert BLP to PNG
cargo run --example convert_from_blp input.blp output.png

# Extract all mipmaps from BLP
cargo run --example extract_mipmaps input.blp output_dir/

# Inspect BLP metadata
cargo run --example inspect_blp input.blp
```

## UI Viewer

The project includes a native GUI viewer built with [egui](https://github.com/emilk/egui):

```bash
# Run the UI viewer
cargo run --bin blp-ui -- path/to/texture.blp

# Or build and install
cargo install --path . --bin blp-ui
blp-ui texture.blp
```

**Features:**
- Drag & drop support
- Mipmap level preview
- Zoom and pan
- Multi-language support (EN, RU, UK, ZH, TC)

## CLI Tool

```bash
# Build the CLI tool
cargo build --release --bin blp-cli

# Or install it
cargo install --path . --bin blp-cli
```

### CLI Commands

**Convert to BLP:**

```bash
blp-cli to-blp input.png output.blp --quality 85
```

**Convert from BLP:**

```bash
blp-cli to-png input.blp output.png
```

## API Overview

### Core Types

- **`AnyImage`** - Universal image container supporting BLP, PNG, JPEG, GIF, PSD, and more
- **`EncodeOptions`** - Output format configuration (PNG, JPEG, BLP)
- **`EncodeMipOptions`** - Mipmap generation control

### Key Methods

```rust
// Load image from buffer
let img = AnyImage::from_buffer(&data)?;

// Get dimensions
let (width, height) = img.dimensions();

// Convert to dynamic image
let dyn_img = img.into_dynamic()?;

// Encode to different formats
let png_data = img.encode(&EncodeOptions::Png { compression: Some(6) })?;
let jpg_data = img.encode(&EncodeOptions::Jpeg { quality: 90 })?;
let blp_data = img.encode(&EncodeOptions::Blp { 
    quality: 85,
    mip_options: None,
    raw: None,
})?;

// Decode all frames/mipmaps
let frames = img.decode_frames()?;
```

### BLP-Specific Functions

```rust
use blp::blp;

// Inspect BLP metadata without decoding pixels
let meta = blp::inspect_buf(&blp_data)?;
println!("Version: {:?}, Size: {}x{}", meta.version, meta.width, meta.height);

// Get header data (JPEG header or palette)
if let Some(header) = blp::header_data(&blp_data) {
    println!("Header size: {} bytes", header.len());
}

// Get raw mipmap data
if let Some(raw) = blp::mip_raw(&blp_data, 0) {
    println!("Mip 0 raw data: {} bytes", raw.len());
}
```

### Mipmap Control

```rust
use blp::any_image::EncodeMipOptions;

// Generate first 4 mipmaps only
let opts = EncodeMipOptions {
    mip_count: Some(4),
    ..Default::default()
};

// Generate until smallest dimension reaches 16px
let opts = EncodeMipOptions {
    min_size: Some(16),
    ..Default::default()
};

// Manual control over each mip level
let opts = EncodeMipOptions {
    specific_mips: Some(vec![true, true, false, true]), // Skip mip level 2
    ..Default::default()
};

// Palette-based compression (experimental)
let opts = EncodeMipOptions {
    quantize_colors: Some(256),  // Use 256-color palette
    quantize_dither: true,       // Enable dithering
    ..Default::default()
};
```

## Format Support

**Input formats:**
- BLP (BLP0, BLP1, BLP2)
- PNG, JPEG, GIF
- PSD (via `psd` crate)
- Any format supported by the `image` crate

**Output formats:**
- BLP (JPEG compression with mipmaps)
- PNG (with compression control)
- JPEG (with quality control)

## Localization

The UI supports multiple languages. Translations are in [`assets/locales/`](assets/locales/).

Supported languages:
- 🇬🇧 English (en)
- 🇷🇺 Russian (ru)
- 🇺🇦 Ukrainian (uk)
- 🇨🇳 Chinese Simplified (zh)
- 🇹🇼 Chinese Traditional (tc)

Contributions for other languages are welcome! You don't need to translate all keys - missing strings fall back to English.

## Building

```bash
# Build library only
cargo build --release

# Build with CLI tool
cargo build --release --bin blp-cli

# Build with UI viewer
cargo build --release --bin blp-ui

# Run tests
cargo test

# Run examples
cargo run --example convert_to_blp -- input.png output.blp
```

## Examples

See the [`examples/`](examples/) directory for complete working examples:

- [`convert_to_blp.rs`](examples/convert_to_blp.rs) - Convert any image to BLP
- [`convert_from_blp.rs`](examples/convert_from_blp.rs) - Convert BLP to PNG/JPEG  
- [`extract_mipmaps.rs`](examples/extract_mipmaps.rs) - Extract all mipmap levels
- [`inspect_blp.rs`](examples/inspect_blp.rs) - Inspect BLP metadata

## Performance

BLP encoding uses [turbojpeg](https://github.com/honzasp/rust-turbojpeg) for high-performance JPEG compression.

Typical performance on modern hardware:
- 1024x1024 with full mipmaps: ~100ms encoding
- BLP → PNG conversion: ~50ms
- Metadata inspection: <1ms (no pixel decoding)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Related Projects

- [WarRaft](https://github.com/WarRaft) - Warcraft III modding toolkit
- [JASS-Tree-sitter-Rust](https://github.com/WarRaft/JASS-Tree-sitter-Rust) - JASS language support
- [BLP Specification](https://github.com/WarRaft/BLP) - Detailed format documentation

---

<p align="center">
  <img src="https://raw.githubusercontent.com/WarRaft/blp-rs/refs/heads/main/preview/logo.png" alt="BLP" width="256"/>
</p>

# Command Line Interface

Lightweight Rust library and tools for reading and writing Blizzard BLP textures.

Quick usage
----------

Decode a `.blp` into PNG using the included example:

```sh
cargo run --example decode_blp -- path/to/input.blp out.png
```

Encode a PNG (or any format supported by the `image` crate) into BLP:

```sh
cargo run --example encode_blp -- in.png out.blp
```

API
---

This crate exposes a small `image`-style extension in `src/blp_image_ext.rs`.
You can use the examples as reference. Minimal API:

- `blp::blp_image_ext::BlpImageDecoder::new(&[u8])` - create decoder
  - `dimensions()` → `(width, height)`
  - `read_image(&mut [u8])` → fills RGBA8 raw pixels (consumes decoder)

- `blp::blp_image_ext::BlpImageEncoder::new(writer)` - create encoder
  - `write_image(buf, width, height, ExtendedColorType)` → writes BLP bytes to writer (consumes encoder)

Notes
-----

- The examples show how to use the decoder/encoder with `image` crate helpers.
- The `image` crate does not support runtime registration of formats in this version; the examples instantiate the decoder/encoder directly.

License
-------

MIT

- **CLI-only** (`--features "cli"`)
- **UI+CLI** (`--features "cli ui"`) – the CLI plus a native GUI viewer

The UI feature always requires CLI, so `ui` cannot be enabled alone.

---

## Usage

```text
blp [PATH]
blp <COMMAND>
```

- In **CLI-only builds**, `[PATH]` performs a *sanity probe*: it checks whether the file is a valid BLP.

    - Success → exit code **0**
    - Failure → exit code **3**

- In **UI+CLI builds**, `[PATH]` launches the native GUI viewer with that file (useful for “Open With…” integration).

If a `<COMMAND>` is provided, it always takes precedence over `[PATH]`.

---

## Commands

### `to-blp`

Convert an image into BLP format.

```text
blp to-blp <INPUT> [OUTPUT] [OPTIONS]
```

- **`<INPUT>`** – input file, usually a PNG
- **`[OUTPUT]`** – optional output path. If not specified, the extension will be replaced with `.blp`

**Options:**

- `--mips <MASK...>`  
  Explicit mipmap mask as a sequence of 0/1 values (length 1–16).

  By default **all mip levels are enabled**.
    - `0` disables a mip level.
    - `1` keeps a mip level enabled (mainly serving to position zeros).

  Example: `--mips 1 0 1 1` → all levels stay enabled except the second one, which is disabled.

- `--mips-limit <N>`  
  Limit the number of generated mip levels (1–16).  
  All levels after `N` are forced to `false`, overriding `--mips` if both are given.

- `-q, --quality <Q>`  
  JPEG quality (1–100).  
  Default: **100**.

---

### `to-png`

Convert a BLP texture into PNG format.

```text
blp to-png <INPUT> [OUTPUT]
```

- **`<INPUT>`** – input file, must be BLP
- **`[OUTPUT]`** – optional output path. If not specified, the extension will be replaced with `.png`

---

## Examples

Check if a BLP file is valid (CLI-only):

```bash
blp MyTexture.blp
echo $?   # → 0 if valid, 3 if invalid
```

Convert PNG to BLP with a custom mip mask:

```bash
blp to-blp input.png --mips 1 0 1 1 -q 85
# disables only the second mip level, all others remain enabled
```

```bash
blp to-blp input.png --mips-limit 4
# keeps only mip levels: 1 (base), 2, 3, 4
# disables levels:        5–16 (if they would exist)
# equivalent to:          --mips 1 1 1 1 0 0 0 0 0 0 0 0 0 0 0 0
```

Convert BLP to PNG:

```bash
blp to-png input.blp output.png
```

Open BLP in GUI (UI+CLI build):

```bash
blp MyTexture.blp
```

# Localization

All localization files are stored in [assets/locales](https://github.com/WarRaft/blp-rs/tree/main/assets/locales).  
You are welcome to contribute a translation in your own language using whatever workflow is most convenient for you, and
I will include it in the program.

It is **not required** to translate every key: any missing strings will automatically fall back to the default English (
`en`) localization. This means you can start small and expand the translation over time without breaking anything.


<p align="center">
  <img src="https://raw.githubusercontent.com/WarRaft/blp-rs/refs/heads/main/preview/logo.png" alt="BLP"/>
</p>