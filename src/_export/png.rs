use crate::blp::{Blp, MAX_MIPS};
use crate::error::error::BlpError;
use image::DynamicImage;
use std::fs;
use std::path::Path;

impl Blp {
    /// Save the given frame (by index) as PNG to out_path.
    /// For BLP this will call a targeted decode of the frame; for other
    /// formats the frame image is obtained from the original buffer.
    pub fn export_png_frame(&self, idx: usize, buf: &[u8], out_path: &Path) -> Result<(), BlpError> {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        // Decode only the requested frame to minimize work
        let mut mask = [false; MAX_MIPS];
        if idx < mask.len() { mask[idx] = true; }
        let decoded = self.decode(buf, &mask)?;
        let rgba = decoded.get(idx).and_then(|opt| opt.as_ref()).ok_or_else(|| BlpError::new("error-export_png").with_arg("msg", "no RGBA decoded for frame"))?;
        DynamicImage::ImageRgba8(rgba.clone()).save(out_path)?;
        Ok(())
    }
}
