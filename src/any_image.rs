use crate::blp::{self, Blp, Frame};
use crate::error::error::BlpError;
use crate::format_detector::FormatDetector;
use crate::gif::Gif;
use crate::psd::PsdImage;
use image::GenericImageView;
use image::{DynamicImage, RgbaImage};

/// AnyImage is a convenience wrapper that accepts an in-memory buffer of
/// unknown image format and exposes a small, user-friendly API.
///
/// Supported inputs: BLP (preferred detection), standard formats supported
/// by the `image` crate (PNG, JPEG, GIF, ...), and PSD (via `psd` crate).
#[derive(Debug, Clone)]
pub struct AnyImage {
    /// Type-specific data (BLP headers, GIF frames metadata, PSD dims, etc.)
    pub data: AnyImageData,
    /// Original buffer given to the loader
    pub buf: Vec<u8>,
    /// Every image has frames (metainfo). For single-frame images this contains one entry.
    pub frames: Vec<Frame>,
}

/// Type-specific inner data for `AnyImage`.
#[derive(Debug, Clone)]
pub enum AnyImageData {
    Blp(Blp),      // header-only BLP representation
    Gif(Gif),      // GIF metadata; frames still in AnyImage.frames
    Psd(PsdImage), // PSD metadata; frames still in AnyImage.frames
    Image,         // regular single-frame image — frames[0] contains dims
}

/// Trait to detect & parse headers cheaply for supported formats.
/// Implementations should be light and must not store the payload (heavy bytes)
/// — only metadata and pointers into `AnyImage.frames` if needed.
// FormatDetector trait lives in `src/format_detector.rs` to avoid circular deps.
// `FormatDetector` now lives in `src/format_detector.rs` and is used by
// the modules (`blp`, `gif`, `psd`) to implement cheap detection + header parsing.

impl AnyImage {
    /// Try to build AnyImage from a byte buffer.
    /// BLP signature is checked first to avoid double-parsing; then `image` is tried,
    /// then PSD as a last fallback.
    pub fn from_buffer(buf: &[u8]) -> Result<Self, BlpError> {
        // Use trait-based detectors — explicit ordering matters (BLP first)
        if Blp::detect(buf) {
            let (blp_hdr, frames) = blp::parse_header(buf)?;
            return Ok(AnyImage { data: AnyImageData::Blp(blp_hdr), buf: buf.to_vec(), frames });
        }

        if Gif::detect(buf) {
            let (gif_meta, frames) = Gif::parse_header(buf)?;
            return Ok(AnyImage { data: AnyImageData::Gif(gif_meta), buf: buf.to_vec(), frames });
        }

        if PsdImage::detect(buf) {
            let (psd_meta, frames) = PsdImage::parse_header(buf)?;
            return Ok(AnyImage { data: AnyImageData::Psd(psd_meta), buf: buf.to_vec(), frames });
        }

        // Other image formats (single frame)
        if let Ok(dynimg) = image::load_from_memory(buf) {
            let (w, h) = dynimg.dimensions();
            let frame = Frame { width: w, height: h, offset: 0, length: buf.len() };
            return Ok(AnyImage { data: AnyImageData::Image, buf: buf.to_vec(), frames: vec![frame] });
        }

        Err(BlpError::new("unsupported-format"))
    }

    /// Return the image width in pixels.
    pub fn width(&self) -> u32 {
        match &self.data {
            AnyImageData::Blp(b) => b.width,
            _ => self
                .frames
                .get(0)
                .map(|f| f.width)
                .unwrap_or(0),
        }
    }

    pub fn height(&self) -> u32 {
        match &self.data {
            AnyImageData::Blp(b) => b.height,
            _ => self
                .frames
                .get(0)
                .map(|f| f.height)
                .unwrap_or(0),
        }
    }

    /// Convert into a single `DynamicImage`. For BLP this returns the first mip
    /// (decoded on demand). Consumes self.
    /// Decode and return the first frame as DynamicImage.
    pub fn into_dynamic(self) -> Result<DynamicImage, BlpError> {
        match self.data {
            AnyImageData::Blp(_) => open(&self.buf),
            AnyImageData::Gif(_) => {
                // GIF: return the first decoded frame as DynamicImage
                let frames = Gif::decode_frames(&self.buf)?;
                if let Some(first) = frames.into_iter().next() {
                    Ok(DynamicImage::ImageRgba8(first))
                } else {
                    Err(BlpError::new("error-gif-no-frame"))
                }
            }
            AnyImageData::Psd(_) => open(&self.buf),
            AnyImageData::Image => open(&self.buf),
        }
    }

    /// Produce a Vec of mipmaps (owned `RgbaImage`s).
    /// For BLP: return the full decoded mip chain. For regular images: generate
    /// a mip chain by successive downscaling of the base image.
    /// Decode and return all frames as RgbaImage (for multi-frame or mipmaps).
    pub fn decode_frames(&self) -> Result<Vec<RgbaImage>, BlpError> {
        match &self.data {
            AnyImageData::Blp(_) => blp::open_mipmaps(&self.buf),
            AnyImageData::Gif(_) => Gif::decode_frames(&self.buf),
            AnyImageData::Psd(_) | AnyImageData::Image => {
                let img = image::load_from_memory(&self.buf)?.to_rgba8();
                Ok(vec![img])
            }
        }
    }

    /// Decode a single frame by index. For BLP, it returns the selected mipmap.
    pub fn decode_frame(&self, idx: usize) -> Result<RgbaImage, BlpError> {
        match &self.data {
            AnyImageData::Blp(b) => {
                // Use BLP internal decoders targeted to the single index
                let frame = self
                    .frames
                    .get(idx)
                    .ok_or_else(|| BlpError::new("error-frame-oob").with_arg("idx", idx as u32))?;
                if frame.length == 0 {
                    return Err(BlpError::new("error-blp-no-mipmap"));
                }
                match b.texture_type {
                    blp::TextureType::JPEG => blp::decode::decode_jpeg_frame(b, frame, &self.buf),
                    blp::TextureType::PALETTE => blp::decode::decode_direct_frame(b, frame, &self.buf),
                }
            }
            AnyImageData::Gif(_) => {
                // use the gif module for GIF frames
                Gif::decode_frame(&self.buf, idx)
            }
            AnyImageData::Psd(_) => {
                // PSD metadata is stored in AnyImageData::Psd, but decode uses our psd module.
                // PSD is single-frame: use psd module
                PsdImage::decode_frame(&self.buf, idx)
            }
            AnyImageData::Image => {
                // single-frame
                let img = image::load_from_memory(&self.buf)?.to_rgba8();
                if idx == 0 { Ok(img) } else { Err(BlpError::new("error-frame-oob").with_arg("idx", idx as u32)) }
            }
        }
    }

    /// If this is a BLP, return metadata; otherwise None.
    /// Return parsed BLP header as the new `blp::Blp` struct if buffer is BLP.
    pub fn blp_meta(&self) -> Option<Blp> {
        match &self.data {
            AnyImageData::Blp(b) => Some(b.clone()),
            _ => None,
        }
    }

    /// For BLP images return the shared JPEG header slice if present.
    pub fn shared_jpeg_header(&self) -> Option<&[u8]> {
        match &self.data {
            AnyImageData::Blp(_) => blp::shared_jpeg_header(&self.buf),
            _ => None,
        }
    }

    /// For BLP images return raw mip payload for index.
    pub fn mip_raw(&self, idx: usize) -> Option<&[u8]> {
        match &self.data {
            AnyImageData::Blp(_) => blp::mip_raw(&self.buf, idx),
            _ => None,
        }
    }
}

/// Decode any supported buffer into a `DynamicImage`.
/// Mirrors previous `src/_from::decode_to_rgba` and `open`.
pub fn decode_to_rgba(buf: &[u8]) -> Result<DynamicImage, BlpError> {
    // BLP detection first
    if Blp::detect(buf) {
        let (blp, frames) = blp::parse_header(buf)?;
        if let Some(frame) = frames.get(0) {
            if frame.length > 0 {
                let img = match blp.texture_type {
                    blp::TextureType::JPEG => blp::decode::decode_jpeg_frame(&blp, frame, buf)?,
                    blp::TextureType::PALETTE => blp::decode::decode_direct_frame(&blp, frame, buf)?,
                };
                return Ok(DynamicImage::ImageRgba8(img));
            }
        }
        return Err(BlpError::new("error-blp-no-mipmap"));
    }

    // PSD special-case
    if PsdImage::detect(buf) {
        return PsdImage::decode_as_dynamic(buf);
    }

    image::load_from_memory(buf).map_err(|_| BlpError::new("error-image-load"))
}

/// Alias to `decode_to_rgba` for compatibility with previous `open` name.
pub fn open(buf: &[u8]) -> Result<DynamicImage, BlpError> {
    decode_to_rgba(buf)
}
