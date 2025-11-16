use crate::error::error::BlpError;
use crate::blp::Blp;
use image::RgbaImage;
use std::fs;
use std::path::Path;

impl Blp {
    pub fn export_blp(&self, frames: &[crate::blp::Frame], out_path: &Path, quality: u8, mip_visible: &[bool], frame_images: &[Option<RgbaImage>]) -> Result<(), BlpError> {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let ctx = self.encode_blp(quality, mip_visible, frames, frame_images)?;

        fs::write(out_path, &ctx.bytes)?;
        Ok(())
    }
}

impl Blp {
    /// Save the given frame (by index) as PNG to out_path.
    /// For BLP this will call a targeted decode of the frame; for other
    /// formats the frame image is obtained from the original buffer.
    pub fn export_png_frame(&self, frames: &[crate::blp::Frame], idx: usize, buf: &[u8], out_path: &Path) -> Result<(), BlpError> {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        // Decode only the requested frame to minimize work
        let mut mask = [false; crate::blp::MAX_MIPS];
        if idx < mask.len() { mask[idx] = true; }
        let decoded = self.decode(frames, buf, &mask)?;
        let rgba = decoded.get(idx).and_then(|opt| opt.as_ref()).ok_or_else(|| BlpError::new("error-export_png").with_arg("msg", "no RGBA decoded for frame"))?;
        image::DynamicImage::ImageRgba8(rgba.clone()).save(out_path)?;
        Ok(())
    }

    /// Export a given frame (by index) as a raw JPEG file by concatenating
    /// the shared JPEG header with the frame tail. Requires the original
    /// BLP buffer `buf` (the same that was parsed).
    pub fn export_jpg_frame(&self, frames: &[crate::blp::Frame], idx: usize, buf: &[u8], out_path: &Path) -> Result<(), BlpError> {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        // This method only makes sense for JPEG-based BLP
        if self.texture_type != crate::blp::TextureType::JPEG {
            return Err(BlpError::new("export-jpg.not-jpeg"));
        }

        // Shared header
        let h_off = self.header.offset;
        let h_len = self.header.length;
        if h_len == 0 || h_off.checked_add(h_len).is_none() || h_off + h_len > buf.len() {
            return Err(BlpError::new("export-jpg.header.oob")
                .with_arg("offset", h_off as u32)
                .with_arg("length", h_len as u32)
                .with_arg("buf_len", buf.len() as u32));
        }
        let header_bytes = &buf[h_off..h_off + h_len];

        let frame = frames.get(idx).ok_or_else(|| BlpError::new("export-jpg.frame_oob"))?;
        let off = frame.offset;
        let len = frame.length;
        if len == 0 || off.checked_add(len).is_none() || off + len > buf.len() {
            return Err(BlpError::new("export-jpg.mip.oob")
                .with_arg("offset", off as u32)
                .with_arg("length", len as u32)
                .with_arg("buf_len", buf.len() as u32));
        }
        let tail = &buf[off..off + len];

        let mut full = Vec::with_capacity(header_bytes.len() + tail.len());
        full.extend_from_slice(header_bytes);
        full.extend_from_slice(tail);

        fs::write(out_path, &full)?;
        Ok(())
    }
}
