use crate::{BlpError, FormatDetector, Frame, ImageDecoder};
use image::{ImageFormat, ImageReader};
use std::io::Cursor;

/// A JPEG image.
#[derive(Debug, Clone)]
pub struct Jpg {
    pub width: u32,
    pub height: u32,
}

impl FormatDetector for Jpg {
    fn detect(buf: &[u8]) -> bool {
        buf.starts_with(&[0xFF, 0xD8, 0xFF])
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        let (width, height) = ImageReader::with_format(Cursor::new(buf), ImageFormat::Jpeg)
            .into_dimensions()
            .map_err(|e| BlpError::new("jpg.parse").push_std(e))?;
        Ok((Jpg { width, height }, vec![Frame { width, height, offset: 0, length: buf.len() }]))
    }
}

impl ImageDecoder for Jpg {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        image::load_from_memory_with_format(buf, ImageFormat::Jpeg).map_err(|e| BlpError::new("jpg.decode").push_std(e))
    }
}
