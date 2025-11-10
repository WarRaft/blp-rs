use crate::_blp_image::BlpImage;
use crate::_error::error::BlpError;
use crate::_decode::decode_to_rgba;

/// Inspect image bytes and return its declared pixel dimensions without full materialization.
pub fn inspect_image_dimensions(buf: &[u8]) -> Result<(u32, u32), BlpError> {
    let img = BlpImage::from_buf_image(buf)?;
    Ok((img.width, img.height))
}

/// Load any supported image (PNG, JPEG, PSD, ...) and return it as DynamicImage.
pub fn load_image_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    decode_to_rgba(buf)
}
