use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use image::{Rgba, RgbaImage};
use jpeg_decoder::{Decoder, PixelFormat};
use std::io::Cursor;

impl ImageBlp {
    /// JPEG path:
    /// - Takes a shared JPEG header from `[header_offset .. header_offset+header_length)`.
    /// - For each mip, concatenates `[header][tail]` → full JPEG, then decodes it.
    /// - Respects `mip_visible`: if `mip_visible[i] == false`, the mip is skipped (image stays `None`).
    ///   Missing indices in `mip_visible` are treated as `true`.
    pub(crate) fn decode_jpeg(&mut self, buf: &[u8], mip_visible: &[bool]) -> Result<(), BlpError> {
        // --- Validate header range and slice it out ---
        let h_off = self.header_offset;
        let h_len = self.header_length;
        if h_off.checked_add(h_len).is_none() || h_off + h_len > buf.len() {
            return Err(BlpError::new("jpeg.header.oob"));
        }
        let header_bytes = &buf[h_off..h_off + h_len];

        // If alpha_bits == 0 we force opaque alpha channel when reconstructing RGBA.
        let force_opaque = self.alpha_bits == 0;

        // --- Walk over mip chain ---
        for i in 0..self.mipmaps.len() {
            // Visibility gate: missing entry → treated as `true`.
            let visible = mip_visible
                .get(i)
                .copied()
                .unwrap_or(true);
            if !visible {
                // Do not materialize pixels for this mip.
                self.mipmaps[i].image = None;
                continue;
            }

            let off = self.mipmaps[i].offset;
            let len = self.mipmaps[i].length;

            // Skip empty mips or invalid ranges safely.
            if len == 0 {
                continue;
            }
            if off.checked_add(len).is_none() || off + len > buf.len() {
                continue;
            }

            // --- Build a full JPEG stream: [shared header][tail for this mip] ---
            let tail = &buf[off..off + len];
            let mut full = Vec::with_capacity(header_bytes.len() + tail.len());
            full.extend_from_slice(header_bytes);
            full.extend_from_slice(tail);

            // --- Decode JPEG ---
            let mut dec = Decoder::new(Cursor::new(&full));
            dec.read_info().map_err(|e| {
                BlpError::from(e)
                    .with_arg("phase", "read_info")
                    .with_arg("mip", i as u32)
            })?;

            let info = dec
                .info()
                .ok_or_else(|| BlpError::new("jpeg.meta.missing").with_arg("mip", i as u32))?;

            let (w, h) = (info.width as u32, info.height as u32);
            let pixels = dec.decode().map_err(|e| {
                BlpError::from(e)
                    .with_arg("phase", "decode")
                    .with_arg("mip", i as u32)
            })?;

            // --- Reconstruct RGBA ---
            let mut img = RgbaImage::new(w, h);
            match info.pixel_format {
                PixelFormat::CMYK32 => {
                    // Expect 4 bytes per pixel: C, M, Y, K
                    if pixels.len() != (w as usize * h as usize * 4) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "CMYK32")
                            .with_arg("mip", i as u32));
                    }
                    for (p, px) in img.pixels_mut().enumerate() {
                        let idx = p * 4;
                        let c = pixels[idx + 0];
                        let m = pixels[idx + 1];
                        let y = pixels[idx + 2];
                        let k = pixels[idx + 3];
                        // Alpha from K (unless forced opaque). Colors inverted from CMY.
                        let a = if force_opaque { 255 } else { 255u8.saturating_sub(k) };
                        *px = Rgba([
                            255u8.saturating_sub(y), // R
                            255u8.saturating_sub(m), // G
                            255u8.saturating_sub(c), // B
                            a,
                        ]);
                    }
                }
                PixelFormat::RGB24 => {
                    // Expect 3 bytes per pixel
                    if pixels.len() != (w as usize * h as usize * 3) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "RGB24")
                            .with_arg("mip", i as u32));
                    }

                    // Fast path (no color transform): pixels are B,G,R in this decoder layout
                    if option_env!("NEVER").is_none() {
                        for (p, px) in img.pixels_mut().enumerate() {
                            let idx = p * 3;
                            *px = Rgba([
                                pixels[idx + 2], // R
                                pixels[idx + 1], // G
                                pixels[idx + 0], // B
                                255,
                            ]);
                        }
                    } else {
                        // Alternative path if you want to pack as YCbCr (kept from your code)
                        for (p, px) in img.pixels_mut().enumerate() {
                            let idx = p * 3;
                            let (r, g, b) = (
                                pixels[idx + 2] as f32, //
                                pixels[idx + 1] as f32,
                                pixels[idx + 0] as f32,
                            );
                            let y = (0.2990 * r + 0.5870 * g + 0.1140 * b)
                                .round()
                                .clamp(0.0, 255.0) as u8;
                            let cb = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b)
                                .round()
                                .clamp(0.0, 255.0) as u8;
                            let cr = (128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b)
                                .round()
                                .clamp(0.0, 255.0) as u8;

                            *px = Rgba([cb, cr, y, 255]);
                        }
                    }
                }
                PixelFormat::L8 => {
                    // 1 byte per pixel (luminance)
                    if pixels.len() != (w as usize * h as usize) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "L8")
                            .with_arg("mip", i as u32));
                    }
                    for (p, px) in img.pixels_mut().enumerate() {
                        let l = pixels[p];
                        *px = Rgba([l, l, l, 255]);
                    }
                }
                PixelFormat::L16 => {
                    // 2 bytes per pixel (big-endian luminance)
                    if pixels.len() != (w as usize * h as usize * 2) {
                        return Err(BlpError::new("jpeg.size.mismatch")
                            .with_arg("fmt", "L16")
                            .with_arg("mip", i as u32));
                    }
                    for (chunk, px) in pixels
                        .chunks_exact(2)
                        .zip(img.pixels_mut())
                    {
                        let l16 = u16::from_be_bytes([chunk[0], chunk[1]]);
                        let l8 = (l16 / 257) as u8; // downscale 16→8
                        *px = Rgba([l8, l8, l8, 255]);
                    }
                }
            }

            // --- Store image into the matching mip level ---
            if self.mipmaps[i].width == w && self.mipmaps[i].height == h {
                self.mipmaps[i].image = Some(img);
            } else if let Some(level) = (0..self.mipmaps.len()).find(|&lvl| self.mipmaps[lvl].width == w && self.mipmaps[lvl].height == h) {
                self.mipmaps[level].image = Some(img);
            }
        }

        Ok(())
    }
}
