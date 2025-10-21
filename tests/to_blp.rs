// src/tests/to_blp.rs
// Юнит-тест: PNG -> BLP (encode) и запуск UI. Минимум шума.

#[cfg(test)]
pub mod to_blp {
    use blp::core::image::ImageBlp;
    use blp::error::error::BlpError;
    use std::fs;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    #[test]
    fn to_blp_encode_and_run_ui() -> Result<(), BlpError> {
        // --- пути ---
        let root = Path::new("/Users/nazarpunk/Downloads/_blp");
        //let a_png = root.join("bb.png");
        let a_png = root.join("logo.png");

        let b_blp = root.join("bb.blp");
        assert!(a_png.exists(), "Missing input PNG: {}", a_png.display());

        // --- PNG -> ImageBlp (разметка + декодирование) ---
        let png_bytes = fs::read(&a_png)?;
        let mut img = ImageBlp::from_buf(&png_bytes)?;
        img.decode(&png_bytes, &[])?;

        // --- encode BLP ---
        let quality = 85u8;
        let ctx = match catch_unwind(AssertUnwindSafe(|| img.encode_blp(quality, &[]))) {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => panic!("encode_blp error: {e}"),
            Err(_) => panic!("encode_blp panicked"),
        };

        // --- write .blp ---
        if let Some(parent) = b_blp.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&b_blp, &ctx.bytes)?;

        // --- run UI (detached) ---
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Command::new("cargo")
            .arg("run")
            .arg("--release")
            .arg("--bin")
            .arg("blp-ui")
            .arg("--features")
            .arg("cli ui")
            .arg("--")
            .arg(b_blp.to_string_lossy().to_string())
            .current_dir(&crate_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start UI");

        Ok(())
    }
}
