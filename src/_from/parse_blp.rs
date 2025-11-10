use crate::_blp_image::{BlpImage, BlpMeta};
use crate::_error::error::BlpError;

/// Parse BLP bytes and return a `BlpImage` instance.
pub fn parse_blp_image(buf: &[u8]) -> Result<BlpImage, BlpError> {
    // Reuse existing parser
    Ok(BlpImage::from_buf_blp(buf)?)
}

/// Inspect BLP bytes and return metadata (`BlpMeta`) without materializing pixels.
pub fn parse_blp_meta(buf: &[u8]) -> Result<BlpMeta, BlpError> {
    BlpImage::inspect_buf(buf)
}
