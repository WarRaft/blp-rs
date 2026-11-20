# blp-rs

[![Crates.io](https://img.shields.io/crates/v/blp.svg)](https://crates.io/crates/blp)
[![Documentation](https://docs.rs/blp/badge.svg)](https://docs.rs/blp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust library for Blizzard's BLP textures, used by Warcraft III and World
of Warcraft. Part of the [WarRaft toolkit](https://github.com/WarRaft) for
Warcraft III modding.

- **Reads every kind of BLP**: BLP1 (Warcraft III) and BLP2 (World of
  Warcraft), stored as JPEG, palette (1, 4 or 8 bits of alpha), DXT1,
  DXT3, DXT5 or raw BGRA. BLP0 (the Warcraft III beta) keeps its mips in
  separate files; `Blp::decode_data` decodes them.
- **Writes BLP1 JPEG** with any choice of mips, from any input image.
- **Any input**: PNG, JPEG, GIF, PSD and everything else the
  [`image`](https://github.com/image-rs/image) crate reads, behind one API.
- **Safe on bad files**: a truncated or corrupt texture is an error, never a
  panic.

## Installation

```toml
[dependencies]
blp = "1.0"
```

| feature   | default | what it adds                                                               |
|-----------|---------|----------------------------------------------------------------------------|
| `encode`  | yes     | writing BLP; builds libjpeg-turbo from source (needs cmake and a C compiler) |
| `psd`     | yes     | Photoshop documents as input                                               |
| `formats` | yes     | every format `image` reads (AVIF, BMP, DDS, EXR, TGA, TIFF, WebP, …)       |

Without any of them (`default-features = false`) the crate is pure Rust,
reads BLP, PNG, JPEG and GIF, and writes PNG and JPEG — enough for a
viewer.

## Reading a BLP

```rust
use blp::Blp;

let buf = std::fs::read("texture.blp")?;
let (blp, mips) = Blp::parse(&buf)?;
println!("{:?} {}, {}x{}", blp.version, blp.encoding()?, blp.width, blp.height);

for (i, mip) in mips.iter().enumerate() {
    // A mip the file doesn't have has length 0 and doesn't decode.
    if mip.length > 0 {
        blp.decode(mip, &buf)?.save(format!("mip_{i}.png"))?;
    }
}
```

`Blp::parse` reads only the header: the version, the encoding, the alpha
depth, the size, and where each mip and the shared data (the JPEG header,
or the palette) are. Mip `i` is `width >> i` by `height >> i`, at least 1.
`Blp::decode` turns one mip into an `RgbaImage`.

## Converting

`AnyImage` reads any supported format and converts it to another:

```rust
use blp::{AnyImage, EncodeMipOptions, EncodeOptions};

let img = AnyImage::from_buffer(&std::fs::read("input.png")?)?;

let blp = img.encode(&EncodeOptions::Blp {
    quality: 85,
    mip_options: Some(EncodeMipOptions { min_size: Some(4), ..Default::default() }),
    raw: None,
})?;
std::fs::write("output.blp", &blp)?;

let png = AnyImage::from_buffer(&blp)?.encode(&EncodeOptions::Png { compression: Some(6) })?;
```

- `dimensions()`: the size (a BLP's base mip).
- `decode_frame(i)`, `decode_frames()`: a BLP's mips, a GIF's frames.
- `to_dynamic()`: the image a viewer shows first, as `image::DynamicImage`.
- `encode(&EncodeOptions)`: PNG (`compression` 0–9), JPEG (`quality`
  0–100), or BLP. BLP output is scaled up to power-of-two sides.
  `raw: Some(i)` instead gives a JPEG BLP's mip `i` as stored, without
  re-encoding.

`EncodeMipOptions` picks the mips; the first option set wins:

| option                         | mips                                        |
|--------------------------------|---------------------------------------------|
| `specific_mips: Some(vec![…])` | mip `i` when `vec[i]` is true (or missing)  |
| `min_size: Some(n)`            | while the smaller side is at least `n`      |
| `mip_count: Some(n)`           | the first `n`                               |
| none                           | all, down to 1×1                            |

## Examples

```bash
cargo run --example convert_to_blp input.png output.blp 85 --min-size 16
cargo run --example convert_from_blp input.blp output.png
cargo run --example extract_mipmaps input.blp output_dir/
cargo run --example inspect_blp input.blp      # or a folder: checks every .blp in it
```

See [`examples/README.md`](examples/README.md).

## Testing

```bash
cargo test
cargo test --no-default-features
```

## Links

- [API documentation](https://docs.rs/blp)
- [BLP format specification](https://github.com/WarRaft/BLP)
- [WarRaft](https://github.com/WarRaft)

## License

MIT — see [LICENSE](LICENSE).

---

<p align="center">
  <img src="https://raw.githubusercontent.com/WarRaft/blp-rs/refs/heads/main/preview/logo.png" alt="BLP Logo" width="256"/>
</p>
