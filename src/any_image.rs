use crate::_blp_image::{BlpImage, MipInfo};
use crate::_blp_image::{BlpMeta, MAX_MIPS};
use crate::_error::error::BlpError;
use image::{DynamicImage, RgbaImage};
use psd::Psd;

/// AnyImage is a convenience wrapper that accepts an in-memory buffer of
/// unknown image format and exposes a small, user-friendly API.
///
/// Supported inputs: BLP (preferred detection), standard formats supported
/// by the `image` crate (PNG, JPEG, GIF, ...), and PSD (via `psd` crate).
#[derive(Debug)]
pub enum AnyImage {
    /// BLP image: contains parsed `BlpImage` and the original bytes (needed to
    /// decode mip tails on demand).
    Blp { img: BlpImage, buf: Vec<u8> },
    /// Regular image loaded via `image::load_from_memory` or PSD converted to DynamicImage.
    Image(DynamicImage),
}

impl AnyImage {
    /// Try to build AnyImage from a byte buffer.
    /// BLP signature is checked first to avoid double-parsing; then `image` is tried,
    /// then PSD as a last fallback.
    pub fn from_buffer(buf: &[u8]) -> Result<Self, BlpError> {
        // Quick BLP signature check (fast, avoids parsing non-BLP with image crate)
        if buf.len() >= 3 && &buf[0..3] == b"BLP" {
            let img = BlpImage::from_buf(buf)?;
            return Ok(AnyImage::Blp { img, buf: buf.to_vec() });
        }

        // Try image crate for common formats
        if let Ok(dynimg) = image::load_from_memory(buf) {
            return Ok(AnyImage::Image(dynimg));
        }

        // PSD fallback (signature 8BPS)
        if buf.len() >= 4 && &buf[0..4] == b"8BPS" {
            let psd = Psd::from_bytes(buf).map_err(|e| BlpError::new("psd-parse").with_arg("error", e.to_string()))?;
            let rgba = psd.rgba();
            let w = psd.width();
            let h = psd.height();
            let imgbuf = RgbaImage::from_raw(w, h, rgba).ok_or_else(|| BlpError::new("psd-invalid-dimensions"))?;
            return Ok(AnyImage::Image(DynamicImage::ImageRgba8(imgbuf)));
        }

        Err(BlpError::new("unsupported-format"))
    }

    /// Return the image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            AnyImage::Blp { img, .. } => img.width,
            AnyImage::Image(d) => d.width(),
        }
    }

    /// Return the image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            AnyImage::Blp { img, .. } => img.height,
            AnyImage::Image(d) => d.height(),
        }
    }

    /// Convert into a single `DynamicImage`. For BLP this returns the first mip
    /// (decoded on demand). Consumes self.
    pub fn into_dynamic(self) -> Result<DynamicImage, BlpError> {
        match self {
            AnyImage::Image(d) => Ok(d),
            AnyImage::Blp { mut img, buf } => {
                // Decode only the first mip
                img.decode(&buf, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])?;
                if let Some(im) = img.mipmaps[0].image.take() {
                    Ok(DynamicImage::ImageRgba8(im))
                } else {
                    Err(BlpError::new("blp-no-mip"))
                }
            }
        }
    }

    /// Produce a Vec of mipmaps (owned `RgbaImage`s).
    /// For BLP: return the full decoded mip chain. For regular images: generate
    /// a mip chain by successive downscaling of the base image.
    pub fn into_mipmaps(self) -> Result<Vec<RgbaImage>, BlpError> {
        match self {
            AnyImage::Image(dynimg) => {
                let mut base = dynimg.to_rgba8();
                let mut out = Vec::new();
                out.push(base.clone());
                for _ in 1..MAX_MIPS {
                    let (w, h) = (base.width(), base.height());
                    if w == 1 && h == 1 {
                        break;
                    }
                    let nw = (w / 2).max(1);
                    let nh = (h / 2).max(1);
                    let next = image::imageops::resize(&base, nw, nh, image::imageops::FilterType::Lanczos3);
                    base = next;
                    out.push(base.clone());
                }
                Ok(out)
            }
            AnyImage::Blp { mut img, buf } => {
                img.decode(&buf, &[true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true])?;
                let mut out = Vec::new();
                for m in img.mipmaps.into_iter().take(MAX_MIPS) {
                    if let Some(rgba) = m.image {
                        out.push(rgba);
                    }
                }
                Ok(out)
            }
        }
    }

    /// If this is a BLP, return metadata; otherwise None.
    pub fn blp_meta(&self) -> Option<BlpMeta> {
        match self {
            AnyImage::Blp { img, .. } => Some(BlpMeta {
                version: img.version,
                texture_type: img.texture_type,
                compression: img.compression,
                alpha_bits: img.alpha_bits,
                alpha_type: img.alpha_type,
                has_mips: img.has_mips,
                width: img.width,
                height: img.height,
                extra: img.extra,
                has_mipmaps: img.has_mipmaps,
                mipmaps: img
                    .mipmaps
                    .iter()
                    .enumerate()
                    .map(|(i, m)| MipInfo { index: i, width: m.width, height: m.height, offset: m.offset, length: m.length })
                    .collect(),
                holes: img.holes,
                header_offset: img.header_offset,
                header_length: img.header_length,
            }),
            AnyImage::Image(_) => None,
        }
    }

    /// For BLP images return the shared JPEG header slice if present.
    pub fn shared_jpeg_header(&self) -> Option<&[u8]> {
        match self {
            AnyImage::Blp { img, buf } => img.shared_jpeg_header(buf),
            _ => None,
        }
    }

    /// For BLP images return raw mip payload for index.
    pub fn mip_raw(&self, idx: usize) -> Option<&[u8]> {
        match self {
            AnyImage::Blp { img, buf } => img.mip_raw(buf, idx),
            _ => None,
        }
    }
}
