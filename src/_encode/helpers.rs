use crate::_blp_image::BlpImage;
use crate::_error::error::BlpError;
use std::vec::Vec;

/// Encode an RGBA buffer into BLP bytes with the given quality and mip visibility.
pub fn encode_rgba_to_blp_bytes(rgba: &[u8], width: u32, height: u32, quality: u8, mip_visible: &[bool]) -> Result<Vec<u8>, BlpError> {
    let bytes = BlpImage::encode_rgba_to_blp(rgba, width, height, quality, mip_visible)?;
    Ok(bytes)
}

use image::RgbaImage;

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
    let mut img = BlpImage::from_rgba(buf, base.width(), base.height())?;

    // Fill subsequent mipmaps up to MAX_MIPS
    for (i, mip) in mips.iter().enumerate().take(crate::_blp_image::MAX_MIPS) {
        if i == 0 { continue; }
        if i >= img.mipmaps.len() { break; }
        let expected_w = img.mipmaps[i].width;
        let expected_h = img.mipmaps[i].height;
        if mip.width() != expected_w || mip.height() != expected_h {
            return Err(BlpError::new("mip.size_mismatch").with_arg("want_w", expected_w).with_arg("want_h", expected_h).with_arg("got_w", mip.width()).with_arg("got_h", mip.height()));
        }
        img.mipmaps[i].image = Some(mip.clone());
    }

    let ctx = img.encode_blp(90, &[true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true])?;
    Ok(ctx.bytes)
}

use crate::_encode::EncodeOptions;

/// Encode with options and return full Ctx (bytes and metadata).
pub fn encode_with_options(img: &BlpImage, opts: &EncodeOptions) -> Result<crate::_encode::blp::Ctx, BlpError> {
    opts.validate()?;

    // Build mip_visible mask according to options
    let mut mip_visible = vec![true; crate::_blp_image::MAX_MIPS];
    match &opts.mip {
        crate::_encode::options::MipSelection::Auto => { /* leave as-is */ }
        crate::_encode::options::MipSelection::Count(n) => {
            for i in 0..mip_visible.len() { mip_visible[i] = i < *n; }
        }
        crate::_encode::options::MipSelection::Explicit(v) => {
            for i in 0..mip_visible.len() { mip_visible[i] = v.get(i).copied().unwrap_or(false); }
        }
    }

    // Delegate to the existing method which handles resampling for SourceKind::Image
    let ctx = img.encode_blp(opts.quality, &mip_visible)?;
    Ok(ctx)
}
