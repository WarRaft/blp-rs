use crate::blp::Blp;
use crate::_error::error::BlpError;
use std::vec::Vec;
use image::RgbaImage;

/// Encode an RGBA buffer into BLP bytes with the given quality and mip visibility.
pub fn encode_rgba_to_blp_bytes(rgba: &[u8], width: u32, height: u32, quality: u8, mip_visible: &[bool]) -> Result<Vec<u8>, BlpError> {
    let img = crate::blp::from_rgba(rgba, width, height)?;
    // Provide base image as frame 0
    let base = image::ImageBuffer::from_raw(width, height, rgba.to_vec()).ok_or_else(|| BlpError::new("error-rgba-image-creation"))?;
    let mut frame_images: Vec<Option<RgbaImage>> = vec![None; img.frames.len()];
    frame_images[0] = Some(base);
    let ctx = img.encode_blp(quality, mip_visible, &frame_images)?;
    let bytes = ctx.bytes;
    Ok(bytes)
}


/// Encode an explicit chain of mipmaps into BLP bytes. The first image is the base level.
/// The images must follow expected mip dimensions (power-of-two cover rules will be applied
/// by `BlpImage::encode_blp` if the source was originally an image), but this helper will
/// validate sizes and return an error on mismatch.
pub fn encode_mipmaps_to_blp_bytes(mips: &[RgbaImage], quality: u8) -> Result<Vec<u8>, BlpError> {
    if mips.is_empty() {
        return Err(BlpError::new("error-empty-mips"));
    }
    let base = &mips[0];
    let buf = base.as_raw();
    let mut img = crate::blp::from_rgba(buf, base.width(), base.height())?;

    // Fill subsequent mipmaps up to MAX_MIPS
    // Validate subsequent mip sizes
    for (i, mip) in mips.iter().enumerate().take(crate::blp::MAX_MIPS) {
        if i == 0 { continue; }
        if i >= img.frames.len() { break; }
        let expected_w = img.frames[i].width;
        let expected_h = img.frames[i].height;
        if mip.width() != expected_w || mip.height() != expected_h {
            return Err(BlpError::new("mip.size_mismatch").with_arg("want_w", expected_w).with_arg("want_h", expected_h).with_arg("got_w", mip.width()).with_arg("got_h", mip.height()));
        }
    }

    // Build images slice: fill with provided mip images at corresponding indices
    let mut framed: Vec<Option<RgbaImage>> = vec![None; img.frames.len()];
    for (i, mip) in mips.iter().enumerate().take(crate::blp::MAX_MIPS) {
        if i == 0 { continue; }
        if i >= img.frames.len() { break; }
        framed[i] = Some(mip.clone());
    }
    // also provide base image for index 0
    framed[0] = Some(base.clone());
    let ctx = img.encode_blp(90, &[true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true], &framed)?;
    Ok(ctx.bytes)
}

use crate::_encode::EncodeOptions;

/// Encode with options and return full Ctx (bytes and metadata).
pub fn encode_with_options(img: &Blp, opts: &EncodeOptions) -> Result<crate::_encode::blp::Ctx, BlpError> {
    opts.validate()?;

    // Build mip_visible mask according to options
    let mut mip_visible = vec![true; crate::blp::MAX_MIPS];
    match &opts.mip {
        crate::_encode::options::MipSelection::Auto => { /* leave as-is */ }
        crate::_encode::options::MipSelection::Count(n) => {
            for i in 0..mip_visible.len() { mip_visible[i] = i < *n; }
        }
        crate::_encode::options::MipSelection::Explicit(v) => {
            for i in 0..mip_visible.len() { mip_visible[i] = v.get(i).copied().unwrap_or(false); }
        }
    }

    // Delegate to the existing method (BlpImage encoder assumes BLP inputs).
    let ctx = img.encode_blp(opts.quality, &mip_visible, &[])?;
    Ok(ctx)
}
