use crate::error::error::BlpError;
use crate::blp::Frame;
use image::{RgbaImage, AnimationDecoder};
use std::io::Cursor;
use crate::traits::FormatDetector;

/// Small header-only structure for GIFs. GIFs are multi-frame, so we produce
/// one `Frame` metadata per frame to align with the rest of the codebase.
#[derive(Debug, Clone)]
pub struct Gif {
    pub width: u32,
    pub height: u32,
}

impl Gif {
    /// Parse the GIF into frame metadata (dimensions + placeholder offsets).
    pub(crate) fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(buf))?;
        let frames = decoder.into_frames();
        let collected = frames.collect_frames()?;
        let meta = collected
            .iter()
            .map(|f| {
                let (w, h) = f.buffer().dimensions();
                Frame { width: w, height: h, offset: 0, length: buf.len() }
            })
            .collect();
        let (w, h) = if let Some(first) = collected.get(0) {
            let (w, h) = first.buffer().dimensions();
            (w, h)
        } else { (0, 0) };
        Ok((Gif { width: w, height: h }, meta))
    }

    /// Fully decode all frames in the GIF payload into owned RGBA images.
    pub(crate) fn decode_frames(buf: &[u8]) -> Result<Vec<RgbaImage>, BlpError> {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(buf))?;
        let frames = decoder.into_frames().collect_frames()?;
        Ok(frames.into_iter().map(|f| f.into_buffer()).collect())
    }

    /// Decode one frame index.
    pub(crate) fn decode_frame(buf: &[u8], idx: usize) -> Result<RgbaImage, BlpError> {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(buf))?;
        let frames = decoder.into_frames().collect_frames()?;
        if let Some(f) = frames.into_iter().nth(idx) {
            Ok(f.into_buffer())
        } else {
            Err(BlpError::new("error-frame-oob").with_arg("idx", idx as u32))
        }
    }
}

impl FormatDetector for Gif {
    fn detect(buf: &[u8]) -> bool {
        buf.len() >= 6 && (&buf[0..6] == b"GIF89a" || &buf[0..6] == b"GIF87a")
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        Gif::parse_header(buf)
    }
}

impl crate::traits::ImageDecoder for Gif {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        // Return the first frame as DynamicImage
        let frames = Gif::decode_frames(buf)?;
        if let Some(first) = frames.into_iter().next() {
            Ok(image::DynamicImage::ImageRgba8(first))
        } else {
            Err(BlpError::new("error-gif-no-frame"))
        }
    }
}
