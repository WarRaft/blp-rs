use crate::blp::Blp;
use crate::error::error::BlpError;
use image::RgbaImage;
use std::vec::Vec;

/// Decode any supported buffer into a `DynamicImage`.
///
/// Attempts to decode BLP first, then PSD, then falls back to standard image formats.
pub fn decode_to_rgba(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    use crate::traits::FormatDetector;
    
    // BLP detection first
    if Blp::detect(buf) {
        let (blp, frames) = crate::blp::parse_header(buf)?;
        if let Some(frame) = frames.get(0) {
            if frame.length > 0 {
                let img = match blp.texture_type {
                    crate::blp::TextureType::JPEG => crate::blp::decode::decode_jpeg_frame(&blp, frame, buf)?,
                    crate::blp::TextureType::PALETTE => crate::blp::decode::decode_palette_frame(&blp, frame, buf)?,
                };
                return Ok(image::DynamicImage::ImageRgba8(img));
            }
        }
        return Err(BlpError::new("error-blp-no-mipmap"));
    }

    // PSD special-case
    if crate::psd::PsdImage::detect(buf) {
        return crate::psd::PsdImage::decode_as_dynamic(buf);
    }

    image::load_from_memory(buf).map_err(|_| BlpError::new("error-image-load"))
}

/// Encode an explicit chain of mipmaps into BLP bytes. The first image is the base level.
pub fn encode_mipmaps_to_blp_bytes(mips: &[RgbaImage], _quality: u8) -> Result<Vec<u8>, BlpError> {
    if mips.is_empty() {
        return Err(BlpError::new("error-empty-mips"));
    }
    let base = &mips[0];
    let buf = base.as_raw();
    let (img, frames) = crate::blp::from_rgba(buf, base.width(), base.height())?;

    for (i, mip) in mips
        .iter()
        .enumerate()
        .take(crate::blp::MAX_MIPS)
    {
        if i == 0 {
            continue;
        }
        if i >= frames.len() {
            break;
        }
        let expected_w = frames[i].width;
        let expected_h = frames[i].height;
        if mip.width() != expected_w || mip.height() != expected_h {
            return Err(BlpError::new("mip.size_mismatch")
                .with_arg("want_w", expected_w)
                .with_arg("want_h", expected_h)
                .with_arg("got_w", mip.width())
                .with_arg("got_h", mip.height()));
        }
    }

    let mut framed: Vec<Option<RgbaImage>> = vec![None; frames.len()];
    for (i, mip) in mips
        .iter()
        .enumerate()
        .take(crate::blp::MAX_MIPS)
    {
        if i == 0 {
            continue;
        }
        if i >= frames.len() {
            break;
        }
        framed[i] = Some(mip.clone());
    }
    framed[0] = Some(base.clone());
    let ctx = img.encode_blp(90, &[true; 16], &frames, &framed)?;
    Ok(ctx.bytes)
}
