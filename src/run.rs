// ===== shared imports =====
#[cfg(any(feature = "cli", feature = "ui"))]
use std::path::PathBuf;

// ===== UI imports =====
#[cfg(feature = "ui")]
use crate::ui::viewer::run_native::run_native;
#[cfg(feature = "cli")]
use {
    crate::cli::command::to_blp::to_blp,
    crate::cli::command::to_png::to_png,
    crate::core::image::MAX_MIPS,
    crate::error::error::BlpError,
    clap::{Parser, Subcommand, error::ErrorKind},
};

// ===== enforce: 'ui' always together with 'cli' =====
#[cfg(all(feature = "ui", not(feature = "cli")))]
compile_error!("Feature 'ui' requires 'cli'. Use either `--features \"cli\"` or `--features \"ui cli\"`. ");

// ======================= CLI subcommands =======================

#[cfg(feature = "cli")]
#[derive(Debug, Subcommand)]
enum Command {
    /// Convert an image into BLP format
    ToBlp {
        /// Input file (e.g. PNG)
        input: PathBuf,
        /// Optional output path. If not specified, the extension will be replaced with .blp
        output: Option<PathBuf>,

        /// Explicit mipmap levels (1–16 numbers).
        #[arg(long = "mips", value_parser = clap::value_parser!(u8).range(1..=16))]
        mips: Vec<u8>,

        /// Limit the number of generated mips (1–16).
        #[arg(long = "mips-limit", value_parser = clap::value_parser!(u8).range(1..=16))]
        mips_limit: Option<u8>,

        /// JPEG quality (1–100).
        #[arg(short = 'q', long = "quality", default_value_t = 100, value_parser = clap::value_parser!(u8).range(1..=100))]
        quality: u8,
    },
    /// Convert a BLP texture into PNG format
    ToPng {
        /// Input file (e.g. BLP)
        input: PathBuf,
        /// Optional output path. If not specified, the extension will be replaced with .png
        output: Option<PathBuf>,
    },
}

/// One unified CLI struct for both builds.
/// - In CLI-only builds, [PATH] triggers a sanity decode (exit 0/3); or use a subcommand.
/// - In UI+CLI builds, [PATH] launches native GUI; or use a subcommand.
/// Command metadata (about/long_about/usage) is specialized with cfg_attr.
#[cfg(feature = "cli")]
#[derive(Debug, Parser)]
#[cfg_attr(all(feature = "cli", not(feature = "ui")), command(name = "blp", version, about = "BLP ↔ PNG converter", long_about = "blp is a command-line utility for converting Warcraft III textures between BLP and PNG formats.", override_usage = "blp [PATH]\nblp <COMMAND>"))]
#[cfg_attr(all(feature = "cli", feature = "ui"), command(name = "blp", version, about = "BLP ↔ PNG converter and simple viewer for Warcraft III textures", long_about = "blp is a command-line utility for converting Warcraft III textures between BLP and PNG formats. It can also launch a native GUI viewer.", override_usage = "blp [PATH]\nblp <COMMAND>"))]
struct Cli {
    /// PATH behavior:
    /// - CLI-only: try to decode this file; success = exit 0, failure = exit 3
    /// - UI+CLI: open native GUI with this file (used by “Open With…”)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Optional subcommand. When present it takes precedence over PATH.
    #[command(subcommand)]
    command: Option<Command>,
}

// ======================= Helpers =======================

#[cfg(feature = "cli")]
fn run_cli_command(cmd: Command) -> Result<(), BlpError> {
    match cmd {
        Command::ToBlp { input, output, mips, mips_limit, quality } => {
            let mut mip_visible = vec![true; MAX_MIPS];

            if !mips.is_empty() {
                for (i, &val) in mips.iter().enumerate() {
                    if i < MAX_MIPS {
                        mip_visible[i] = val != 0;
                    }
                }
            }

            if let Some(limit) = mips_limit {
                let limit = limit as usize;
                for i in limit..MAX_MIPS {
                    mip_visible[i] = false;
                }
            }
            to_blp(&input, output.as_ref(), quality, &mip_visible)
        }
        Command::ToPng { input, output } => to_png(&input, output.as_ref()),
    }
}

/// CLI-only: probe if the file is a valid BLP.
/// Success → exit 0; failure → exit 3.
/// This function always terminates the process.
#[cfg(all(feature = "cli", not(feature = "ui")))]
fn sanity_decode_or_exit(path: PathBuf) -> ! {
    use crate::core::image::ImageBlp;
    use std::fs;

    // Read file
    let data = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read error: {e}");
            std::process::exit(3);
        }
    };

    // Minimal probe: try to parse the BLP header
    if let Err(e) = ImageBlp::from_buf(&data) {
        eprintln!("{e}");
        std::process::exit(3);
    }

    // If we reach here → file is parseable
    std::process::exit(0);
}

// ======================= Entry point ========================

// Unified entry point for both CLI-only and UI+CLI builds
#[cfg(any(feature = "cli", feature = "ui"))]
pub fn run() -> Result<(), BlpError> {
    // Unified CLI parsing:
    // - Help/Version → print and return Ok(())
    // - Other errors → print and exit with code 2
    let Some(cli) = (match Cli::try_parse() {
        Ok(cli) => Some(cli),
        Err(e) => {
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    let _ = e.print(); // graceful 0
                    None
                }
                _ => {
                    let _ = e.print();
                    std::process::exit(e.exit_code()); // usually 2
                }
            }
        }
    }) else {
        return Ok(());
    };

    // ===== UI + CLI build =====
    #[cfg(all(feature = "cli", feature = "ui"))]
    {
        return if let Some(cmd) = cli.command { run_cli_command(cmd) } else { run_native(cli.path) };
    }

    // ===== CLI-only build =====
    #[cfg(all(feature = "cli", not(feature = "ui")))]
    {
        match (cli.path, cli.command) {
            // Single PATH → sanity decode (process exits inside helper)
            (Some(p), None) => {
                sanity_decode_or_exit(p);
            }
            // Subcommand without PATH
            (None, Some(cmd)) => run_cli_command(cmd),
            // Both PATH and subcommand → prefer subcommand (ignore PATH)
            (Some(_), Some(cmd)) => run_cli_command(cmd),
            // Neither PATH nor subcommand → print error and exit with code 2
            (None, None) => {
                eprintln!("error: a PATH or a subcommand is required\n\nUse --help for more information.");
                std::process::exit(2);
            }
        }
    }
}
