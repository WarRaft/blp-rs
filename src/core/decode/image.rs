use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use image::imageops::{FilterType, crop_imm, resize};
use image::{self};

impl ImageBlp {
    /// External image path:
    /// 1) Scale-to-cover without aspect distortion to (target_w, target_h).
    /// 2) Center-crop to exactly (target_w, target_h).
    /// 3) Generate mip chain, honoring `mip_visible` flags:
    ///    - If `mip_visible[i] == false` → we do NOT materialize pixels for mip i (image stays `None`).
    ///    - Missing indices in `mip_visible` are treated as `true`.
    pub(crate) fn decode_image(&mut self, buf: &[u8], mip_visible: &[bool]) -> Result<(), BlpError> {
        // --- Decode source into RGBA8 ---
        let src = image::load_from_memory(buf)
            .map_err(|e| BlpError::new("image.decode").with_arg("msg", e.to_string()))?
            .to_rgba8();

        // Target size (at least 1×1).
        let (tw, th) = (self.width.max(1), self.height.max(1));
        let (sw, sh) = src.dimensions();

        if sw == 0 || sh == 0 {
            return Err(BlpError::new("image.zero_dims")
                .with_arg("w", sw)
                .with_arg("h", sh));
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
