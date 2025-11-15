// `crate::blp` is not directly used, but left for clarity of module's intent.
use crate::format_detector::FormatDetector;
use crate::error::error::BlpError;
use image;
use psd::Psd;

const MAX_POW2: u32 = 8192; // adjust upper bound if needed

/// Checks if buffer is a PSD file by signature
fn is_psd_file(buf: &[u8]) -> bool {
    // PSD files start with "8BPS" signature
    buf.len() >= 4 && &buf[0..4] == b"8BPS"
}

/// Gets PSD file dimensions without full decoding
fn get_psd_dimensions(buf: &[u8]) -> Result<(u32, u32), BlpError> {
    let psd = Psd::from_bytes(buf).map_err(|e| BlpError::new("error-psd-parse").with_arg("error", e.to_string()))?;

    let width = psd.width();
    let height = psd.height();

    if width == 0 || height == 0 {
        return Err(BlpError::new("error-psd-invalid-dimensions")
            .with_arg("width", width)
            .with_arg("height", height));
    }

    Ok((width, height))
}

fn pow2_list_up_to(max_v: u32) -> Vec<u32> {
    let mut v = 1u32;
    let mut out = Vec::new();
    while v <= max_v {
        out.push(v);
        if v == u32::MAX / 2 {
            break;
        }
        v <<= 1;
    }
    out
}

/// Choose target frame (W*, H*) — powers of two.
/// Criteria (lexicographically):
///   1) minimum scale s = max(W*/w0, H*/h0) (no distortion, "minimal stretch")
///   2) minimum difference in aspect ratio |(W*/H*) - (w0/h0)|
///   3) minimum area W* * H*
/// Returns (W*, H*).
pub(crate) fn pick_pow2_cover(w0: u32, h0: u32) -> (u32, u32) {
    debug_assert!(w0 > 0 && h0 > 0);
    let ws = pow2_list_up_to(MAX_POW2);
    let hs = pow2_list_up_to(MAX_POW2);

    let w0f = w0 as f64;
    let h0f = h0 as f64;
    let ar0 = w0f / h0f;

    let mut best = None::<(f64, f64, u64, u32, u32)>; // (s, ar_diff, area, W, H)

    for &ww in &ws {
        // if very small powers of two — skip obviously smaller than source:
        // BUT we allow "sub-frames" smaller than source (this will increase s), so don't filter.
        for &hh in &hs {
            let s = (ww as f64 / w0f).max(hh as f64 / h0f); // cover scale
            if s < 1.0 {
                // Must cover the frame — if s<1, image won't cover the frame.
                continue;
            }
            let ar = ww as f64 / hh as f64;
            let ar_diff = (ar - ar0).abs();
            let area = (ww as u64) * (hh as u64);

            let cand = (s, ar_diff, area, ww, hh);
            match best {
                None => best = Some(cand),
                Some(cur) => {
                    // comparison: s, then ar_diff, then area
                    if cand.0 < cur.0 || (cand.0 == cur.0 && (cand.1 < cur.1 || (cand.1 == cur.1 && cand.2 < cur.2))) {
                        best = Some(cand);
                    }
                }
            }
        }
    }

    if let Some((_s, _ard, _area, ww, hh)) = best { (ww, hh) } else { (w0, h0) }
}

/// Create mipmaps for the given base dimensions.
/// first_image: Some(image) for the first mipmap if available, None otherwise.
/// Create a `Blp` from a generic image buffer by decoding it to RGBA and
/// constructing a power-of-two frame chain with the base image as frame 0.
pub fn from_buf_image(buf: &[u8]) -> Result<crate::blp::Blp, BlpError> {
    // Decode PSD specially
    let dynimg = if is_psd_file(buf) {
        let psd = Psd::from_bytes(buf).map_err(|e| BlpError::new("error-psd-parse").with_arg("error", e.to_string()))?;
        let rgba = psd.rgba();
        let w = psd.width();
        let h = psd.height();
        image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(w, h, rgba).ok_or_else(|| BlpError::new("psd-invalid-dimensions"))?)
    } else {
        image::load_from_memory(buf).map_err(|_| BlpError::new("error-image-load"))?
    };

    let rgba = dynimg.to_rgba8();
    let (w, h) = rgba.dimensions();
    let raw = rgba.into_raw();
    crate::blp::from_rgba(&raw, w, h)
}

/// Decode any supported image format to DynamicImage.
///
/// For BLP files: returns the first mipmap level.
/// For other formats (PNG, JPG, PSD, etc.): decodes the full image.
pub fn decode_to_rgba(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    // Check if it's a BLP file using canonical detector
    if crate::blp::Blp::detect(buf) {
        // Parse header into Blp
        let mut blp = crate::blp::parse_header(buf)?;
        // Decode only first frame/mip
        let decoded = match blp.texture_type {
            crate::blp::TextureType::JPEG => crate::blp::decode::decode_jpeg_to_mipmaps(&blp, buf, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])?,
            crate::blp::TextureType::PALETTE => crate::blp::decode::decode_direct_to_mipmaps(&blp, buf, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])?,
        };
        // First present decoded image
        if let Some(Some(img)) = decoded.into_iter().next() {
            Ok(image::DynamicImage::ImageRgba8(img))
        } else {
            Err(BlpError::new("error-blp-no-mipmap"))
        }
    } else {
        // Decode as regular image (PNG, JPG, PSD, etc.)
        // PSD special-case: delegate to PSD wrapper
            if crate::psd::PsdImage::detect(buf) {
            crate::psd::PsdImage::decode_as_dynamic(buf)
        } else {
            image::load_from_memory(buf).map_err(|_| BlpError::new("error-image-load"))
        }
    }
}
