use crate::_blp_image::BlpImage;
use crate::_error::error::BlpError;
use std::path::Path;

/// Export a BlpImage to the filesystem (convenience wrapper around `export_blp`).
pub fn export_blp_to_path(img: &BlpImage, out_path: &Path, quality: u8, mip_visible: &[bool]) -> Result<(), BlpError> {
    img.export_blp(out_path, quality, mip_visible)
}
