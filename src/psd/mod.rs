use crate::_error::error::BlpError;
use crate::blp::Frame;
use image::RgbaImage;
use psd::Psd;

/// PSD wrapper for our codebase. PSD is single-frame; we expose a Frame chain
/// with one frame (base) so all formats follow the "frames everywhere"
/// convention.
#[derive(Debug, Clone)]
pub struct PsdImage {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<Frame>,
}

impl PsdImage {
    pub fn parse_header(buf: &[u8]) -> Result<Self, BlpError> {
        let psd = Psd::from_bytes(buf).map_err(|e| BlpError::new("psd-parse").with_arg("error", e.to_string()))?;
        let w = psd.width();
        let h = psd.height();
        let frames = vec![Frame { width: w, height: h, offset: 0, length: buf.len() }];
        Ok(PsdImage { width: w, height: h, frames })
    }

    pub fn decode_frames(buf: &[u8]) -> Result<Vec<RgbaImage>, BlpError> {
        let psd = Psd::from_bytes(buf).map_err(|e| BlpError::new("psd-parse").with_arg("error", e.to_string()))?;
        let rgba = psd.rgba();
        let (w, h) = (psd.width(), psd.height());
        let img = image::ImageBuffer::from_raw(w, h, rgba).ok_or_else(|| BlpError::new("error-psd-invalid-dimensions").with_arg("width", w).with_arg("height", h))?;
        Ok(vec![image::DynamicImage::ImageRgba8(img).to_rgba8()])
    }

    pub fn decode_frame(buf: &[u8], idx: usize) -> Result<RgbaImage, BlpError> {
        if idx > 0 { return Err(BlpError::new("error-frame-oob").with_arg("idx", idx as u32)); }
        Ok(Self::decode_frames(buf)?.into_iter().next().unwrap())
    }
}
