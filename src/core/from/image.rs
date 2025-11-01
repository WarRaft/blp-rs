use crate::core::image::{ImageBlp, MAX_MIPS};
use crate::core::mipmap::Mipmap;
use crate::core::types::SourceKind;
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
fn pick_pow2_cover(w0: u32, h0: u32) -> (u32, u32) {
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
fn create_mipmaps(base_w: u32, base_h: u32, first_image: Option<image::RgbaImage>) -> Vec<Mipmap> {
    // How many levels to 1×1 inclusive:
    // floor(log2(max)) + 1  ==  32 - leading_zeros(max)  (for u32)
    let levels = (32 - base_w.max(base_h).leading_zeros()) as usize;

    let mut mipmaps = Vec::with_capacity(MAX_MIPS);
    let (mut w, mut h) = (base_w, base_h);

    for i in 0..MAX_MIPS {
        if i < levels {
            mipmaps.push(Mipmap { width: w, height: h, image: if i == 0 { first_image.clone() } else { None }, offset: 0, length: 0 });
            // halve, but not below 1
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        } else {
            // tail — missing levels
            mipmaps.push(Mipmap::default());
        }
    }

    mipmaps
}

impl ImageBlp {
    /// Lightweight path for "arbitrary image": layout only without RGBA.
    /// Supports both regular image formats (via image library),
    /// and Adobe Photoshop (PSD) files with automatic signature detection.
    ///
    /// 1) Read source dimensions (without full decoding)
    /// 2) Choose target frame (W*,H*) — powers of two by "minimum upscale" and "minimum crop" rule
    /// 3) Form mipmap chain (only width/height), image=None
    ///    Tail after 1×1 filled with 0×0 (not 1×1).
    pub fn from_buf_image(buf: &[u8]) -> Result<Self, BlpError> {
        // Get image dimensions without full decoding
        let (w0, h0) = if is_psd_file(buf) {
            get_psd_dimensions(buf)?
        } else {
            let reader = image::ImageReader::new(std::io::Cursor::new(buf))
                .with_guessed_format()
                .map_err(|_| BlpError::new("error-image-load"))?;
            let dimensions = reader
                .into_dimensions()
                .map_err(|_| BlpError::new("error-image-load"))?;
            dimensions
        };

        if w0 == 0 || h0 == 0 {
            return Err(BlpError::new("error-image-empty")
                .with_arg("width", w0)
                .with_arg("height", h0));
        }

        let (base_w, base_h) = pick_pow2_cover(w0, h0);

        let mipmaps = create_mipmaps(base_w, base_h, None);

        Ok(ImageBlp { width: base_w, height: base_h, mipmaps, source: SourceKind::Image, ..Default::default() })
    }

    /// Create BLP from raw RGBA buffer.
    /// Buffer must be in RGBA format (4 bytes per pixel).
    /// Width and height must match the buffer size.
    pub fn from_rgba_impl(rgba_buf: &[u8], width: u32, height: u32) -> Result<Self, BlpError> {
        if width == 0 || height == 0 {
            return Err(BlpError::new("error-image-empty")
                .with_arg("width", width)
                .with_arg("height", height));
        }

        let expected_size = (width * height * 4) as usize;
        if rgba_buf.len() != expected_size {
            return Err(BlpError::new("error-rgba-buffer-size")
                .with_arg("expected", expected_size)
                .with_arg("actual", rgba_buf.len()));
        }

        // Create RgbaImage from buffer
        let rgba_image = image::RgbaImage::from_raw(width, height, rgba_buf.to_vec()).ok_or_else(|| BlpError::new("error-rgba-image-creation"))?;

        let (base_w, base_h) = pick_pow2_cover(width, height);

        let mipmaps = create_mipmaps(base_w, base_h, Some(rgba_image));

        Ok(ImageBlp { width: base_w, height: base_h, mipmaps, source: SourceKind::Image, ..Default::default() })
    }
}
