use crate::error::error::BlpError;

/// Convert two bytes into u16 big-endian.
#[inline]
pub fn read_be_u16(b: &[u8]) -> Result<u16, BlpError> {
    if b.len() < 2 {
        return Err(BlpError::new("jpeg.len"));
    }
    Ok(((b[0] as u16) << 8) | b[1] as u16)
}
