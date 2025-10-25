This is a pure Rust take on the classic Warcraft III BLP texture format.  
No C glue, no old-school wrappers — just clean Rust that works everywhere: Windows, macOS, Linux.

It has a tiny UI made with [egui](https://github.com/emilk/egui) — drop in a file, and boom, you can view it.  
Under the hood there's a simple Rust library, perfect when you just need BLP decoding inside your own tools.

Oh, and it's part of the WarRaft toolkit — you’ll also find it used together
with [JASS-Tree-sitter-Rust](https://github.com/WarRaft/JASS-Tree-sitter-Rust),  
which brings syntax support, analyzers, and more tooling for Warcraft III modding.

Wanna know how BLP works? Dive into the spec:  
👉 [BLP Specification](https://github.com/WarRaft/BLP)

# Command Line Interface

The `blp` tool can be built in two configurations:

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