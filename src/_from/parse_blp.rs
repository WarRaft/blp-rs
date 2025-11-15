use crate::blp::{self, Blp};
use crate::_error::error::BlpError;

/// Parse BLP bytes and return a lightweight `blp::Blp` header structure.
pub fn parse_blp_image(buf: &[u8]) -> Result<Blp, BlpError> {
    blp::parse_header(buf)
}

/// Inspect BLP bytes and return metadata (currently same as `parse_blp_image`).
pub fn parse_blp_meta(buf: &[u8]) -> Result<Blp, BlpError> {
    blp::parse_header(buf)
}
