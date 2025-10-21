use crate::error::error::BlpError;
use crate::core::image::ImageBlp;
use std::fs;
use std::path::Path;

impl ImageBlp {
    pub fn export_blp(&self, out_path: &Path, quality: u8, mip_visible: &[bool]) -> Result<(), BlpError> {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let ctx = self.encode_blp(quality, mip_visible)?;

        // Сохраняем готовый контейнер
        fs::write(out_path, &ctx.bytes)?;
        Ok(())
    }
}
