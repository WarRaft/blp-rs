# Examples

```bash
cargo run --example <name> -- [arguments]
```

## `convert_to_blp`

Any image (PNG, JPEG, GIF, PSD, …) to BLP1 JPEG with mips, scaled up to
power-of-two sides.

```bash
cargo run --example convert_to_blp input.png output.blp            # quality 85, all mips
cargo run --example convert_to_blp input.jpg output.blp 90         # quality 0–100
cargo run --example convert_to_blp input.png output.blp 85 --min-size 16
```

## `convert_from_blp`

A BLP's base mip to PNG or JPEG, by the output's extension.

```bash
cargo run --example convert_from_blp input.blp output.png
cargo run --example convert_from_blp input.blp output.jpg 90
```

## `extract_mipmaps`

Every mip of a BLP, decoded as PNG into `decoded/`, and for JPEG textures
the stored JPEG as is into `raw/`.

```bash
cargo run --example extract_mipmaps input.blp output_dir/
```

## `inspect_blp`

What a BLP's header says, and whether every mip decodes. Given a folder,
checks every `.blp` in it and below and lists the ones that fail.

```bash
cargo run --example inspect_blp texture.blp
cargo run --release --example inspect_blp path/to/extracted/game/
```
