use crate::{BlpError, FormatDetector, Frame, ImageDecoder};
use psd::Psd;

/// A Photoshop document, read as its composite image.
#[derive(Debug, Clone)]
pub struct PsdImage {
    pub width: u32,
    pub height: u32,
}

fn open(buf: &[u8]) -> Result<Psd, BlpError> {
    Psd::from_bytes(buf).map_err(|e| BlpError::new("psd.parse").with_arg("error", e.to_string()))
}

impl FormatDetector for PsdImage {
    fn detect(buf: &[u8]) -> bool {
        buf.starts_with(b"8BPS")
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        let psd = open(buf)?;
        let (width, height) = (psd.width(), psd.height());
        Ok((PsdImage { width, height }, vec![Frame { width, height, offset: 0, length: buf.len() }]))
    }
}

impl ImageDecoder for PsdImage {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        let psd = open(buf)?;
        let (w, h) = (psd.width(), psd.height());
        image::RgbaImage::from_raw(w, h, psd.rgba())
            .map(image::DynamicImage::ImageRgba8)
            .ok_or_else(|| {
                BlpError::new("psd.size")
                    .with_arg("width", w)
                    .with_arg("height", h)
            })
    }
}
