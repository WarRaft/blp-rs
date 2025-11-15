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
    // BLP-only path: parse header, decode all frames and return them.
    let img = blp::parse_header(buf)?;
    let mip_visible = &[true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true];
    let decoded = match img.texture_type {
        crate::blp::TextureType::JPEG => crate::blp::decode::decode_jpeg_to_mipmaps(&img, buf, mip_visible)?,
        crate::blp::TextureType::PALETTE => crate::blp::decode::decode_direct_to_mipmaps(&img, buf, mip_visible)?,
    };
    let mut out = Vec::new();
    for opt in decoded
        .into_iter()
        .take(crate::blp::MAX_MIPS)
    {
        if let Some(rgba) = opt {
            out.push(rgba);
        }
    }
    Ok(out)
}

/// Open the buffer and return a `DynamicImage`.
/// Uses `decode_to_rgba` which detects and decodes BLP or other supported formats.
pub fn open(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    crate::_from::decode_to_rgba(buf)
}
