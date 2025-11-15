use crate::blp::Blp;
use crate::_error::error::BlpError;

/// Delegate parsing to the canonical `blp::parse_header` implementation.
pub(crate) fn from_buf_blp(buf: &[u8]) -> Result<Blp, BlpError> {
    crate::blp::parse_header(buf)
}
