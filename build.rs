// build.rs
//
// This build script appends a human-readable build section to
// build/build-info.txt on every top-level build. The shell script is expected
// to truncate (or remove) this file before invoking cargo, so Cargo sees the
// file as changed and reruns this build script on the next build.
//
// It also runs the icon generation/embedding logic (build/icons.rs) and writes
// a concise, emoji-rich report into the log.
//
// Requirements:
// - Your shell build driver should do:
//     mkdir -p build
//     : > build/build-info.txt
//     export BLP_BUILD_ID="some-random-id"   # optional
//
// Why the rerun? Cargo only reruns build scripts when watched inputs change.
// We watch build/build-info.txt; your shell truncates it each run, so Cargo
// will rerun this script next time.

#[path = "build/icons.rs"]
mod icons;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use chrono::Utc;
    use std::{env, fs, io::Write, path::Path};

    // 1) Register the log file as a rerun trigger.
    //    The shell truncates it before each build, so Cargo will detect the
    //    change and rerun this script on the next build.
    println!("cargo:rerun-if-changed=build/build-info.txt");
    // Optional: rerun when these env vars change.
    println!("cargo:rerun-if-env-changed=BLP_BUILD_ID");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_DESCRIPTION");
    println!("cargo:rerun-if-env-changed=CARGO_BIN_NAME");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_AUTHORS");

    // 2) Always run icon generation / embedding (it decides per platform).
    //    It returns a human-readable per-OS report (emoji included).
    let icons_report = icons::run_icons()?; // Option<String>

    // === 2.1) Skip log if no BLP_BUILD_ID ===
    let build_id = env::var("BLP_BUILD_ID").unwrap_or_default();
    if build_id.is_empty() {
        return Ok(()); // nothing to append
    }

    // 3) Append a new build section to the human-readable log.
    let log_path = Path::new("build/build-info.txt");
    if let Some(dir) = log_path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut out = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    // Collect build metadata from Cargo env.
    let pkg_name = env::var("CARGO_PKG_NAME").unwrap_or_default();
    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let features: Vec<String> = env::vars()
        .filter(|(k, _)| k.starts_with("CARGO_FEATURE_"))
        .map(|(k, _)| {
            k.trim_start_matches("CARGO_FEATURE_")
                .to_lowercase()
        })
        .collect();

    // Two blank lines to visually separate sections.
    writeln!(out)?;
    writeln!(out)?;

    // ✨ Header (easy to scan)
    writeln!(out, "===== 🛠️  Build @ {} =====", Utc::now())?;
    writeln!(out, "🆔 Build ID : {}", build_id)?;
    writeln!(out, "📦 Package  : {} v{}", pkg_name, pkg_version)?;
    writeln!(out, "🧭 Target OS: {}", target_os)?;
    writeln!(out, "🧩 Features : {:?}", features)?;
    writeln!(out, "----- 🖼️  Icons -----")?;

    // Icons report (or why it was skipped)
    match icons_report {
        Some(report) => {
            // The report itself already contains emojis per-OS branches.
            writeln!(out, "{report}")?;
        }
        None => {
            writeln!(out, "⏭️  Icons: skipped (feature `ui` is disabled)")?;
        }
    }

    Ok(())
}
