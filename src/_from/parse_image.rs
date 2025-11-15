use crate::blp;
use crate::error::error::BlpError;

/// Inspect image bytes and return its declared pixel dimensions without full materialization.
pub fn inspect_image_dimensions(buf: &[u8]) -> Result<(u32, u32), BlpError> {
    // Only BLP inspection is supported here now.
    let h = blp::parse_header(buf)?;
    Ok((h.width, h.height))
}

/// Return all mipmaps as owned `RgbaImage`s.
/// For BLP files this returns the full mip chain. For regular images (PNG/JPG)
/// this will generate a mip chain by downscaling the base image.
pub fn open_mipmaps(buf: &[u8]) -> Result<Vec<image::RgbaImage>, BlpError> {
    crate::blp::open_mipmaps(buf)
}

/// Open the buffer and return a `DynamicImage`.
/// Uses `decode_to_rgba` which detects and decodes BLP or other supported formats.
pub fn open(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    crate::any_image::decode_to_rgba(buf)
}
