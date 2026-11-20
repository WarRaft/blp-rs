use crate::{BlpError, FormatDetector, Frame, ImageDecoder};
use image::{AnimationDecoder, RgbaImage, codecs::gif::GifDecoder};
use std::io::Cursor;

/// A GIF: one [`Frame`] per animation frame, all of its full size.
#[derive(Debug, Clone)]
pub struct Gif {
    pub width: u32,
    pub height: u32,
}

impl Gif {
    /// Every frame, composited, as RGBA.
    pub(crate) fn decode_frames(buf: &[u8]) -> Result<Vec<RgbaImage>, BlpError> {
        let frames = GifDecoder::new(Cursor::new(buf))?
            .into_frames()
            .collect_frames()?;
        Ok(frames
            .into_iter()
            .map(|f| f.into_buffer())
            .collect())
    }
}

impl FormatDetector for Gif {
    fn detect(buf: &[u8]) -> bool {
        buf.starts_with(b"GIF87a") || buf.starts_with(b"GIF89a")
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        let frames = Gif::decode_frames(buf)?;
        let (width, height) = frames
            .first()
            .map_or((0, 0), |f| f.dimensions());
        let meta = frames
            .iter()
            .map(|f| Frame { width: f.width(), height: f.height(), offset: 0, length: buf.len() })
            .collect();
        Ok((Gif { width, height }, meta))
    }
}

impl ImageDecoder for Gif {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        let first = Gif::decode_frames(buf)?
            .into_iter()
            .next()
            .ok_or_else(|| BlpError::new("gif.no-frames"))?;
        Ok(image::DynamicImage::ImageRgba8(first))
    }
}
