use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use std::fs;
use std::path::{Path, PathBuf};

pub fn to_png(input: &Path, output: Option<&PathBuf>) -> Result<(), BlpError> {
    input.try_exists()?;
    let data = fs::read(input)?;
    let mut img = ImageBlp::from_buf(&data).map_err(|e| e.ctx("blp.decode-failed"))?;
    img.decode(&data, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])?;

    let out_path: PathBuf = match output {
        Some(p) => p.clone(),
        None => input.with_extension("png"),
    };

    img.export_png(img.mipmaps.get(0).unwrap(), &out_path)?;
    println!("Saved PNG → {}", out_path.display());
    Ok(())
}
