use crate::blp::Blp;
use crate::error::error::BlpError;
use std::vec::Vec;
use image::RgbaImage;
use crate::blp::EncodeOptions;
use crate::blp::MipSelection;

/// Encode an RGBA buffer into BLP bytes with the given quality and mip visibility.
pub fn encode_rgba_to_blp_bytes(rgba: &[u8], width: u32, height: u32, quality: u8, mip_visible: &[bool]) -> Result<Vec<u8>, BlpError> {
    crate::blp::encode_rgba_to_blp(rgba, width, height, quality, mip_visible)
}

/// Encode an explicit chain of mipmaps into BLP bytes. The first image is the base level.
pub fn encode_mipmaps_to_blp_bytes(mips: &[RgbaImage], _quality: u8) -> Result<Vec<u8>, BlpError> {
    if mips.is_empty() {
        return Err(BlpError::new("error-empty-mips"));
    }
    let base = &mips[0];
    let buf = base.as_raw();
    let (img, frames) = crate::blp::from_rgba(buf, base.width(), base.height())?;

    for (i, mip) in mips.iter().enumerate().take(crate::blp::MAX_MIPS) {
        if i == 0 { continue; }
        if i >= frames.len() { break; }
        let expected_w = frames[i].width;
        let expected_h = frames[i].height;
        if mip.width() != expected_w || mip.height() != expected_h {
            return Err(BlpError::new("mip.size_mismatch").with_arg("want_w", expected_w).with_arg("want_h", expected_h).with_arg("got_w", mip.width()).with_arg("got_h", mip.height()));
        }
    }

    let mut framed: Vec<Option<RgbaImage>> = vec![None; frames.len()];
    for (i, mip) in mips.iter().enumerate().take(crate::blp::MAX_MIPS) {
        if i == 0 { continue; }
        if i >= frames.len() { break; }
        framed[i] = Some(mip.clone());
    }
    framed[0] = Some(base.clone());
    let ctx = img.encode_blp(90, &[true; 16], &frames, &framed)?;
    Ok(ctx.bytes)
}

/// Encode with options and return full Ctx (bytes and metadata).
pub fn encode_with_options(img: &Blp, frames: &[crate::blp::Frame], opts: &EncodeOptions) -> Result<crate::blp::encode::Ctx, BlpError> {
    opts.validate()?;

    let mut mip_visible = vec![true; crate::blp::MAX_MIPS];
    match &opts.mip {
        MipSelection::Auto => { /* leave as-is */ }
        MipSelection::Count(n) => {
            for i in 0..mip_visible.len() { mip_visible[i] = i < *n; }
        }
        MipSelection::Explicit(v) => {
            for i in 0..mip_visible.len() { mip_visible[i] = v.get(i).copied().unwrap_or(false); }
        }
    }

    let ctx = img.encode_blp(opts.quality, &mip_visible, frames, &[])?;
    Ok(ctx)
}
