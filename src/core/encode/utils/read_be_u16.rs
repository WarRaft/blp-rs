use crate::error::error::BlpError;

/// Преобразует два байта в значение u16 в big-endian.
/// Возвращает ошибку, если данных меньше 2.
#[inline]
pub fn read_be_u16(b: &[u8]) -> Result<u16, BlpError> {
    if b.len() < 2 {
        return Err(BlpError::new("jpeg.len"));
    }
    Ok(((b[0] as u16) << 8) | b[1] as u16)
}
