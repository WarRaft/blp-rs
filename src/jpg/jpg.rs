use crate::error::error::BlpError;
use crate::blp;
use image::codecs::jpeg::JpegEncoder;

/// Options for JPEG encoding/extraction.
#[derive(Debug, Clone)]
pub enum JpgOptions {
    Raw,
    Reencode { quality: u8 },
}

pub struct Jpg;

impl Jpg {
    pub fn encode(buf: &[u8], frame_idx: usize, opts: JpgOptions) -> Result<Vec<u8>, BlpError> {
        match opts {
            JpgOptions::Raw => {
                let h = blp::parse_header(buf)?;
                match h.texture_type {
                    blp::TextureType::JPEG => {
                        let hdr = blp::shared_jpeg_header(buf).ok_or_else(|| BlpError::new("jpeg.shared_header_missing"))?;
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
                use image::codecs::jpeg::JpegEncoder;
                use image::ImageEncoder;
                use crate::blp::TextureType;

                if blp::parse_header(buf).is_ok() {
                    let header = blp::parse_header(buf)?;
                    let decoded = match header.texture_type {
                        TextureType::JPEG => blp::decode::decode_jpeg_to_mipmaps(&header, buf, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])?,
                        TextureType::PALETTE => blp::decode::decode_direct_to_mipmaps(&header, buf, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])?,
                    };
                    if let Some(Some(img)) = decoded.get(frame_idx) {
                        let mut out = Vec::new();
                        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
                        let mut enc = JpegEncoder::new_with_quality(&mut out, quality as u8);
                        enc.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8.into())?;
                        Ok(out)
                    } else {
                        Err(BlpError::new("jpeg.reencode-frame-not-found"))
                    }
                } else {
                    let dynimg = crate::_from::decode_to_rgba(buf)?;
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
