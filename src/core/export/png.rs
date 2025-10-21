use crate::core::image::ImageBlp;
use crate::core::mipmap::Mipmap;
use crate::error::error::BlpError;
use image::DynamicImage;
use std::fs;
use std::path::Path;

impl ImageBlp {
    /// Сохранить переданный мип как PNG в out_path.
    /// Требуется, чтобы в мипе уже было `image: Some(RgbaImage)`.
    pub fn export_png(&self, mip: &Mipmap, out_path: &Path) -> Result<(), BlpError> {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let rgba = mip
            .image
            .as_ref()
            .ok_or_else(|| BlpError::new("error-export_png").with_arg("msg", "no RGBA in mip"))?;

        DynamicImage::ImageRgba8(rgba.clone()).save(out_path)?;
        Ok(())
    }
}
