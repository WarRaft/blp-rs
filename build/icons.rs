// build/icons.rs
use std::error::Error;
use std::{fs, path::Path};

/// Returns a human-readable, emoji-rich report for the *target* OS,
/// or None when feature `ui` is OFF (features propagate to build.rs via env).
pub fn run_icons() -> Result<Option<String>, Box<dyn Error>> {
    // Respect the main crate's `ui` feature (#[cfg(feature)] doesn't apply in build scripts).
    if std::env::var_os("CARGO_FEATURE_UI").is_none() {
        return Ok(None);
    }

    let src_icon = Path::new("assets/icon.png");
    let out_dir = Path::new("assets/generated");
    fs::create_dir_all(out_dir)?;

    // IMPORTANT: use target triple from Cargo to dispatch
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| std::env::consts::OS.to_string());

    let report = match target_os.as_str() {
        "windows" => do_windows(src_icon, out_dir)?,
        "macos" => do_macos(src_icon, out_dir)?,
        // treat everything else as Linux/XDG
        _ => do_linux(src_icon, out_dir)?,
    };

    Ok(Some(report))
}

/* ------------------- WINDOWS ------------------- */

fn do_windows(src_icon: &Path, out_dir: &Path) -> Result<String, Box<dyn Error>> {
    use std::io;

    let mut actions = Vec::<String>::new();
    let ico_path = out_dir.join("app.ico");

    // Generate app.ico only if missing and we have a source PNG.
    if !ico_path.exists() && src_icon.exists() {
        let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
        let img = image::load_from_memory(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
            .to_rgba8();
        let (w, h) = (img.width(), img.height());
        if w != h {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("icon.png must be square, got {w}x{h}")).into());
        }
        generate_ico(out_dir, &img, &[16, 24, 32, 48, 64, 128, 256])?;
        actions.push(format!("🧩 Generated app.ico (16,24,32,48,64,128,256) → {}", ico_path.display()));
    } else if ico_path.exists() {
        actions.push(format!("♻️  Reused {}", ico_path.display()));
    } else {
        actions.push("⏭️  Skipped ICO generation (assets/icon.png not found)".into());
    }

    // Always embed VERSIONINFO (and ICON if present).
    actions.push(embed_windows_resources(out_dir));

    Ok(format!("🪟 Windows:\n  - {}", actions.join("\n  - ")))
}

fn generate_ico(out_dir: &Path, img: &image::RgbaImage, sizes: &[u32]) -> Result<(), Box<dyn Error>> {
    use ico::{IconDir, IconImage, ResourceType};
    use std::io;

    let mut dir = IconDir::new(ResourceType::Icon);
    for &s in sizes {
        let resized = image::imageops::resize(img, s, s, image::imageops::FilterType::Lanczos3);
        let ii = IconImage::from_rgba_data(s, s, resized.into_raw());
        let entry = ico::IconDirEntry::encode(&ii).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode ico {s}px: {e}")))?;
        dir.add_entry(entry);
    }
    let path = out_dir.join("app.ico");
    let mut f = fs::File::create(&path).map_err(|e| io::Error::new(e.kind(), format!("create {}: {e}", path.display())))?;
    dir.write(&mut f)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("write {}: {e}", path.display())))?;
    Ok(())
}

fn embed_windows_resources(out_dir: &Path) -> String {
    use std::{env, io, path::Path as StdPath};

    fn normalize_version(v: &str) -> String {
        let mut parts = [0u16; 4];
        let mut i = 0usize;
        for seg in v.split('.') {
            if i >= 4 {
                break;
            }
            let num = seg
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();
            parts[i] = num.parse::<u16>().unwrap_or(0);
            i += 1;
        }
        format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3])
    }

    let pkg_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "app".to_string());
    let pkg_desc = env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| pkg_name.clone());
    let pkg_verraw = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let pkg_auth = env::var("CARGO_PKG_AUTHORS").unwrap_or_default();
    let bin_name = env::var("CARGO_BIN_NAME").unwrap_or_else(|_| pkg_name.clone());
    let ver = normalize_version(&pkg_verraw);

    let company = pkg_auth
        .split(':')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown Company")
        .to_string();

    let ico_path = out_dir.join("app.ico");
    let mut res = winresource::WindowsResource::new();

    if let Some(p) = ico_path.to_str() {
        if StdPath::new(p).exists() {
            res.set_icon(p);
        }
    }

    res.set("FileVersion", &ver);
    res.set("ProductVersion", &ver);
    res.set("FileDescription", &pkg_desc);
    res.set("ProductName", &pkg_name);
    res.set("CompanyName", &company);
    res.set("InternalName", &bin_name);
    res.set("OriginalFilename", &format!("{}.exe", bin_name));
    let legal = if pkg_auth.is_empty() { format!("© {}", company) } else { format!("© {}", pkg_auth) };
    res.set("LegalCopyright", &legal);

    // Use en-US for VERSIONINFO; app itself handles i18n.
    res.set_language(0x0409);

    match res.compile() {
        Ok(_) => format!("🔐 Embedded VERSIONINFO (Product='{}', FileVersion='{}'){}", pkg_name, ver, if ico_path.exists() { " + ICON" } else { " (no ICON)" }),
        Err(e) => {
            let mut msg = format!("💥 Embedding resources FAILED: {e}");
            if e.kind() == io::ErrorKind::NotFound {
                msg.push_str(" [hint: ensure `rc.exe` on MSVC, or `llvm-rc`/`windres` for cross]");
            }
            msg
        }
    }
}

/* ------------------- macOS ------------------- */

fn do_macos(src_icon: &Path, out_dir: &Path) -> Result<String, Box<dyn Error>> {
    use icns::{IconFamily, IconType, Image as IcnsImage, PixelFormat};
    use std::io;

    let icns_path = out_dir.join("AppIcon.icns");

    if icns_path.exists() || !src_icon.exists() {
        let msg = if icns_path.exists() {
            format!("♻️  Reused {}", icns_path.display())
        } else {
            "⏭️  Skipped .icns generation (assets/icon.png not found)".to_string()
        };
        return Ok(format!(" macOS:\n  - {msg}"));
    }

    let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
    let img = image::load_from_memory(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
        .to_rgba8();

    let mut family = IconFamily::new();

    fn kinds(base: u32) -> &'static [IconType] {
        match base {
            16 => &[IconType::RGBA32_16x16, IconType::RGBA32_16x16_2x],
            32 => &[IconType::RGBA32_32x32, IconType::RGBA32_32x32_2x],
            64 => &[IconType::RGBA32_64x64],
            128 => &[IconType::RGBA32_128x128, IconType::RGBA32_128x128_2x],
            256 => &[IconType::RGBA32_256x256, IconType::RGBA32_256x256_2x],
            512 => &[IconType::RGBA32_512x512, IconType::RGBA32_512x512_2x],
            _ => &[],
        }
    }
    fn px(kind: IconType) -> Option<u32> {
        Some(match kind {
            IconType::RGBA32_16x16 => 16,
            IconType::RGBA32_16x16_2x => 32,
            IconType::RGBA32_32x32 => 32,
            IconType::RGBA32_32x32_2x => 64,
            IconType::RGBA32_64x64 => 64,
            IconType::RGBA32_128x128 => 128,
            IconType::RGBA32_128x128_2x => 256,
            IconType::RGBA32_256x256 => 256,
            IconType::RGBA32_256x256_2x => 512,
            IconType::RGBA32_512x512 => 512,
            IconType::RGBA32_512x512_2x => 1024,
            _ => return None,
        })
    }

    for &base in &[16, 32, 64, 128, 256, 512] {
        for &kind in kinds(base) {
            if let Some(s) = px(kind) {
                let resized = image::imageops::resize(&img, s, s, image::imageops::FilterType::Lanczos3);
                let raw = resized.into_raw();
                let icns_img = IcnsImage::from_data(PixelFormat::RGBA, s, s, raw).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("icns {s}px: {e}")))?;
                family
                    .add_icon_with_type(&icns_img, kind)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("add slice {s}px: {e}")))?;
            }
        }
    }

    let mut f = fs::File::create(&icns_path).map_err(|e| io::Error::new(e.kind(), format!("create {}: {e}", icns_path.display())))?;
    family
        .write(&mut f)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("write {}: {e}", icns_path.display())))?;

    Ok(format!(" macOS:\n  - 🧩 Generated {}", icns_path.display()))
}

/* ------------------- LINUX / OTHERS ------------------- */

fn do_linux(src_icon: &Path, out_dir: &Path) -> Result<String, Box<dyn Error>> {
    use std::io;
    use std::path::PathBuf;

    let sizes = &[16, 32, 48, 64, 128, 256, 512];
    let targets: Vec<PathBuf> = sizes
        .iter()
        .map(|s| out_dir.join(format!("icons/hicolor/{}x{}/apps/blp.png", s, s)))
        .collect();

    if targets.iter().all(|p| p.exists()) || !src_icon.exists() {
        let msg = if targets.iter().all(|p| p.exists()) {
            "♻️  Reused hicolor icon set".to_string()
        } else {
            "⏭️  Skipped hicolor generation (assets/icon.png not found)".to_string()
        };
        return Ok(format!("🐧 Linux:\n  - {msg}"));
    }

    let data = fs::read(src_icon).map_err(|e| io::Error::new(e.kind(), format!("read {}: {e}", src_icon.display())))?;
    let img = image::load_from_memory(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode {}: {e}", src_icon.display())))?
        .to_rgba8();

    for &s in sizes {
        let resized = image::imageops::resize(&img, s, s, image::imageops::FilterType::Lanczos3);
        let dir = out_dir.join(format!("icons/hicolor/{}x{}/apps", s, s));
        fs::create_dir_all(&dir)?;
        let path = dir.join("blp.png");

        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(resized)
            .write_to(&mut io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode {}: {e}", path.display())))?;

        let mut f = fs::File::create(&path).map_err(|e| io::Error::new(e.kind(), format!("create {}: {e}", path.display())))?;
        use std::io::Write;
        f.write_all(&buf)
            .map_err(|e| io::Error::new(e.kind(), format!("write {}: {e}", path.display())))?;
    }

    Ok("🐧 Linux:\n  - 🧩 Generated hicolor icon set (16..512 px)".to_string())
}
