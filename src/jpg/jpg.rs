use crate::blp;
use crate::blp::Frame;
use crate::error::error::BlpError;
use crate::traits::{FormatDetector, ImageDecoder};
use image::GenericImageView;

/// Options for JPEG encoding/extraction.
#[derive(Debug, Clone)]
pub enum JpgOptions {
    Raw,
    Reencode { quality: u8 },
}

#[derive(Debug, Clone)]
pub struct Jpg {
    pub width: u32,
    pub height: u32,
}

impl FormatDetector for Jpg {
    fn detect(buf: &[u8]) -> bool {
        // JPEG magic: FF D8 FF
        buf.len() >= 3 && buf[0] == 0xFF && buf[1] == 0xD8 && buf[2] == 0xFF
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        if !Self::detect(buf) {
            return Err(BlpError::new("jpg.invalid_signature"));
        }

        // Parse JPEG to get dimensions
        let img = image::load_from_memory(buf)
            .map_err(|e| BlpError::new("jpg.parse_failed").push_std(e))?;
        
        let (width, height) = img.dimensions();
        let jpg = Jpg { width, height };
        
        // JPEG is single-frame
        let frames = vec![Frame { width, height, offset: 0, length: buf.len() }];
        
        Ok((jpg, frames))
    }
}

impl ImageDecoder for Jpg {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        image::load_from_memory(buf)
            .map_err(|e| BlpError::new("jpg.decode_failed").push_std(e))
    }
}

impl Jpg {
    pub(crate) fn encode(buf: &[u8], frame_idx: usize, opts: JpgOptions) -> Result<Vec<u8>, BlpError> {
        match opts {
            JpgOptions::Raw => {
                let (h, _frames) = blp::parse_header(buf)?;
                match h.texture_type {
                    blp::TextureType::JPEG => {
                        let hdr = blp::header_data(buf).ok_or_else(|| BlpError::new("jpeg.shared_header_missing"))?;
                        let mip = blp::mip_raw(buf, frame_idx).ok_or_else(|| BlpError::new("jpeg.mip_missing"))?;
                        let mut out = Vec::with_capacity(hdr.len() + mip.len());
                        out.extend_from_slice(hdr);
                        out.extend_from_slice(mip);
                        Ok(out)
                    }
                    _ => Err(BlpError::new("jpeg.raw-not-blp-jpeg")),
                }
            }
            JpgOptions::Reencode { quality } => {
                use crate::blp::TextureType;
                use image::codecs::jpeg::JpegEncoder;

                if blp::parse_header(buf).is_ok() {
                    let (header, frames) = blp::parse_header(buf)?;
                    let frame = frames
                        .get(frame_idx)
                        .ok_or_else(|| BlpError::new("jpeg.reencode-frame-not-found"))?;
                    if frame.length == 0 {
                        return Err(BlpError::new("jpeg.reencode-frame-empty"));
                    }
                    let img = match header.texture_type {
                        TextureType::JPEG => blp::decode::decode_jpeg_frame(&header, frame, buf)?,
                        TextureType::PALETTE => blp::decode::decode_palette_frame(&header, frame, buf)?,
                    };
                    let mut out = Vec::new();
                    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
                    let mut enc = JpegEncoder::new_with_quality(&mut out, quality as u8);
                    enc.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8.into())?;
                    Ok(out)
                } else {
                    // No quality arg — detect the external format and convert.
                    use crate::traits::{FormatDetector, ImageDecoder};

                    let dynimg = if blp::Blp::detect(buf) {
                        blp::Blp::into_dynamic(buf)?
                    } else if crate::psd::PsdImage::detect(buf) {
                        crate::psd::PsdImage::into_dynamic(buf)?
                    } else {
                        image::load_from_memory(buf).map_err(|_| BlpError::new("error-image-load"))?
                    };

                    let img = dynimg.to_rgba8();
                    let mut out = Vec::new();
                    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
                    let mut enc = JpegEncoder::new_with_quality(&mut out, quality as u8);
                    enc.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8.into())?;
                    Ok(out)
                }
            }
        }
    }
}
