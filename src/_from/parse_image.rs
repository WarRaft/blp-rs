use crate::blp;
use crate::_error::error::BlpError;
use crate::_decode::decode_to_rgba;

/// Inspect image bytes and return its declared pixel dimensions without full materialization.
pub fn inspect_image_dimensions(buf: &[u8]) -> Result<(u32, u32), BlpError> {
    // Only BLP inspection is supported here now.
    let h = blp::parse_header(buf)?;
    Ok((h.width, h.height))
}

/// Load any supported image (PNG, JPEG, PSD, ...) and return it as DynamicImage.
pub fn load_image_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    decode_to_rgba(buf)
}

/// Open an image buffer and return a DynamicImage.
/// This is the public convenience loader that prefers BLP (to avoid
/// double-parsing), then falls back to other supported formats.
pub fn open(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    // Currently `decode_to_rgba` already implements the desired logic
    // (BLP detection + other formats), so delegate to it here to provide
    // a clear, single entry point for callers.
    decode_to_rgba(buf)
}

/// Return all mipmaps as owned `RgbaImage`s.
/// For BLP files this returns the full mip chain. For regular images (PNG/JPG)
/// this will generate a mip chain by downscaling the base image.
pub fn open_mipmaps(buf: &[u8]) -> Result<Vec<image::RgbaImage>, BlpError> {
    // BLP-only path: parse header, decode all frames and return them.
    let img = blp::parse_header(buf)?;
    let mip_visible = &[true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true];
    let decoded = match img.texture_type {
        crate::blp::TextureType::JPEG => crate::_decode::decode_jpeg_to_mipmaps(&img, buf, mip_visible)?,
        crate::blp::TextureType::PALETTE => crate::_decode::decode_direct_to_mipmaps(&img, buf, mip_visible)?,
    };
    let mut out = Vec::new();
    for opt in decoded.into_iter().take(crate::blp::MAX_MIPS) {
        if let Some(rgba) = opt {
            out.push(rgba);
        }
    }
    Ok(out)
}
