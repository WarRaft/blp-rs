use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use image::imageops::{FilterType, crop_imm, resize};
use image::{self};
use psd::Psd;

/// Checks if buffer is a PSD file by signature
fn is_psd_file(buf: &[u8]) -> bool {
    // PSD files start with "8BPS" signature
    buf.len() >= 4 && &buf[0..4] == b"8BPS"
}

/// Checks if buffer is a BLP file by signature
fn is_blp_file(buf: &[u8]) -> bool {
    // BLP files start with "BLP1" or "BLP2" signature
    buf.len() >= 4 && (&buf[0..4] == b"BLP1" || &buf[0..4] == b"BLP2")
}

/// Loads PSD file and converts it to DynamicImage
///
/// Extracts from PSD file:
/// - Composite (final) image in RGBA format
/// - Image dimensions (width and height)
/// - Automatically processes all layers, effects, and blend modes
///
/// Note: The psd library provides a ready "flattened" image,
/// which is the result of compositing all visible layers taking into account
/// their blend modes, transparency, and other effects.
fn load_psd_as_image(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    let psd = Psd::from_bytes(buf).map_err(|e| BlpError::new("error-psd-parse").with_arg("error", e.to_string()))?;

    // Get composite RGBA image (result of all layers)
    let rgba_data = psd.rgba();
    let width = psd.width();
    let height = psd.height();

    // Additional PSD info (for debugging if needed)
    #[cfg(debug_assertions)]
    {
        eprintln!("PSD Info: {}x{}, {} bytes of RGBA data", width, height, rgba_data.len());
    }

    // Create ImageBuffer from RGBA data
    let img_buf = image::ImageBuffer::from_raw(width, height, rgba_data).ok_or_else(|| {
        BlpError::new("error-psd-invalid-dimensions")
            .with_arg("width", width)
            .with_arg("height", height)
    })?;

    Ok(image::DynamicImage::ImageRgba8(img_buf))
}

/// Common function to decode image (PSD or regular formats) into DynamicImage.
/// Used by both decode_image() and decode_to_rgba() to avoid code duplication.
fn decode_image_common(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    if is_psd_file(buf) {
        load_psd_as_image(buf)
    } else {
        image::load_from_memory(buf).map_err(|_| BlpError::new("error-image-load"))
    }
}

impl ImageBlp {
    /// External image path:
    /// 1) Scale-to-cover without aspect distortion to (target_w, target_h).
    /// 2) Center-crop to exactly (target_w, target_h).
    /// 3) Generate mip chain, honoring `mip_visible` flags:
    ///    - If `mip_visible[i] == false` → we do NOT materialize pixels for mip i (image stays `None`).
    ///    - Missing indices in `mip_visible` are treated as `true`.
    pub fn decode_image(&mut self, buf: &[u8], mip_visible: &[bool]) -> Result<(), BlpError> {
        // --- Decode source into RGBA8 ---
        let src = decode_image_common(buf)?;
        let src = src.to_rgba8();

        // Target size (at least 1×1).
        let (tw, th) = (self.width.max(1), self.height.max(1));
        let (sw, sh) = src.dimensions();

        if sw == 0 || sh == 0 {
            return Err(BlpError::new("error-image-empty")
                .with_arg("width", sw)
                .with_arg("height", sh));
        }

        // --- (1) cover-scale: choose the larger scale so the image covers the target area ---
        let sx = tw as f32 / sw as f32;
        let sy = th as f32 / sh as f32;
        let s = sx.max(sy);
        let rw = (sw as f32 * s).ceil() as u32;
        let rh = (sh as f32 * s).ceil() as u32;
        let resized = resize(&src, rw, rh, FilterType::Lanczos3);

        // --- (2) center-crop to exactly (tw, th) ---
        // Guard against underflow with saturating_sub; clamp crop origin into valid range.
        let cx = ((rw.saturating_sub(tw)) / 2).min(rw.saturating_sub(tw));
        let cy = ((rh.saturating_sub(th)) / 2).min(rh.saturating_sub(th));
        let base = crop_imm(&resized, cx, cy, tw, th).to_image();

        // --- (3) build mip chain, honoring `mip_visible` ---
        let mut prev = base;
        let (mut w, mut h) = (tw, th);

        for i in 0..self.mipmaps.len() {
            // Record dimensions for this mip (even if we skip pixels).
            self.mipmaps[i].width = w;
            self.mipmaps[i].height = h;

            // Visibility gate: missing entry → treated as `true`.
            let visible = mip_visible
                .get(i)
                .copied()
                .unwrap_or(true);
            if visible {
                // Materialize RGBA only if requested.
                self.mipmaps[i].image = Some(prev.clone());
            } else {
                self.mipmaps[i].image = None;
            }

            // Stop when we reached 1×1.
            if w == 1 && h == 1 {
                // Optionally clear the rest (keep dims at 1×1 and no pixels).
                for j in (i + 1)..self.mipmaps.len() {
                    self.mipmaps[j].width = 1;
                    self.mipmaps[j].height = 1;
                    self.mipmaps[j].image = None;
                }
                break;
            }

            // Next mip level dims: halve each dimension, clamp to ≥1.
            let next_w = (w / 2).max(1);
            let next_h = (h / 2).max(1);

            // Downscale current level into the next.
            let next_img = resize(&prev, next_w, next_h, FilterType::Lanczos3);

            // Prepare for next iteration.
            prev = next_img;
            w = next_w;
            h = next_h;
        }

        Ok(())
    }
}

/// Decode any supported image format to RGBA image.
///
/// For BLP files: returns the first mipmap level.
/// For other formats (PNG, JPG, PSD, etc.): decodes the full image.
///
/// # Returns
///
/// `DynamicImage` - decoded image that can be converted to RGBA8 or other formats
///
/// # Examples
///
/// ```no_run
/// use blp::core::decode::decode_to_rgba;
///
/// let file_data = std::fs::read("image.png").unwrap();
/// let img = decode_to_rgba(&file_data).unwrap();
/// let rgba = img.to_rgba8();
/// println!("Image: {}x{}", img.width(), img.height());
/// ```
pub fn decode_to_rgba(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
    // Check if it's a BLP file
    if is_blp_file(buf) {
        // Decode BLP and get first mipmap
        let mut blp = ImageBlp::from_buf_blp(buf)?;

        // Decode only first mip - all others are disabled
        blp.decode(buf, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])?;

        // Get first mipmap
        if let Some(img) = blp.mipmaps[0].image.take() {
            Ok(image::DynamicImage::ImageRgba8(img))
        } else {
            Err(BlpError::new("error-blp-no-mipmap"))
        }
    } else {
        // Decode as regular image (PNG, JPG, PSD, etc.)
        decode_image_common(buf)
    }
}
