use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use std::fs;
use std::path::{Path, PathBuf};

pub fn to_blp(input: &Path, output: Option<&PathBuf>, quality: u8, mip_visible: &[bool]) -> Result<(), BlpError> {
    input.try_exists()?;
    let data = fs::read(input)?;
    let mut img = ImageBlp::from_buf(&data)?;
    img.decode(&data, mip_visible)?;

    let out_path: PathBuf = match output {
        Some(p) => p.clone(),
        None => input.with_extension("blp"),
    };

    img.export_blp(&out_path, quality, mip_visible)?;
    println!("Saved BLP → {}", out_path.display());
    Ok(())
}
