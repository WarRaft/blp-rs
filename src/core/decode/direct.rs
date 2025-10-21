use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use byteorder::{LittleEndian, ReadBytesExt};
use image::RgbaImage;
use std::io::{Cursor, Read};

impl ImageBlp {
    /// DIRECT (paletted) decoding.
    ///
    /// Reads palette from the header area and, for each mipmap, reads indices + alpha,
    /// then reconstructs RGBA images.
    ///
    /// - `mip_visible[i] == false` → skip decoding for that mipmap (image stays `None`).
    /// - If `mip_visible` has no entry for index `i`, we treat it as `true`.
    pub(crate) fn decode_direct(&mut self, buf: &[u8], mip_visible: &[bool]) -> Result<(), BlpError> {
        // --- Read palette ---
        // Palette is located at `self.header_offset` with expected length = 256 * 4.
        if self.header_offset + self.header_length > buf.len() {
            return Err(BlpError::new("direct.header.oob"));
        }
        let mut cur = Cursor::new(&buf[..]);
        cur.set_position(self.header_offset as u64);

        let mut palette = [[0u8; 3]; 256];
        for i in 0..256 {
            let color = cur.read_u32::<LittleEndian>()?;
            // Format: R = bits 16..23, G = bits 8..15, B = bits 0..7
            let r = ((color >> 16) & 0xFF) as u8;
            let g = ((color >> 8) & 0xFF) as u8;
            let b = (color & 0xFF) as u8;
            palette[i] = [r, g, b];
        }

        let buf_len = buf.len();
        let alpha_bits = self.alpha_bits;

        // --- Process mipmaps ---
        for i in 0..self.mipmaps.len() {
            // Check if this mipmap should be decoded
            let visible = mip_visible
                .get(i)
                .copied()
                .unwrap_or(true);
            if !visible {
                self.mipmaps[i].image = None;
                continue;
            }

            let off = self.mipmaps[i].offset;
            let len = self.mipmaps[i].length;
            if len == 0 {
                continue; // no data for this mip
            }
            if off.checked_add(len).is_none() || off + len > buf_len {
                continue; // invalid offset/length
            }

            cur.set_position(off as u64);

            let (w, h) = (self.mipmaps[i].width, self.mipmaps[i].height);
            let pixel_count = (w as usize) * (h as usize);

            // --- Read indices (one byte per pixel) ---
            let mut indices = vec![0u8; pixel_count];
            cur.read_exact(&mut indices)
                .map_err(|_| BlpError::new("direct.indices.truncated"))?;

            // --- Read alpha data depending on alpha_bits ---
            let alpha_bytes = match alpha_bits {
                0 => 0,
                1 => (pixel_count + 7) / 8, // 1 bit per pixel
                4 => (pixel_count + 1) / 2, // 4 bits per pixel
                8 => pixel_count,           // 1 byte per pixel
                _ => return Err(BlpError::new("blp.version.invalid").with_arg("msg", "unsupported alpha bits")),
            };
            let mut alpha_raw = vec![0u8; alpha_bytes];
            if alpha_bytes > 0 {
                cur.read_exact(&mut alpha_raw)
                    .map_err(|_| BlpError::new("direct.alpha.truncated"))?;
            }

            // --- Assemble RGBA image ---
            let mut img = RgbaImage::new(w, h);
            for p in 0..pixel_count {
                let idx = indices[p] as usize;
                let [r, g, b] = palette[idx];
                let a = match alpha_bits {
                    0 => 255,
                    1 => {
                        let byte = alpha_raw[p / 8];
                        let bit = (byte >> (p % 8)) & 1;
                        if bit == 1 { 255 } else { 0 }
                    }
                    4 => {
                        let byte = alpha_raw[p / 2];
                        let nibble = if (p & 1) == 0 { byte & 0x0F } else { byte >> 4 };
                        (nibble << 4) | nibble
                    }
                    8 => alpha_raw[p],
                    _ => 255,
                };
                img.get_pixel_mut((p as u32) % w, (p as u32) / w)
                    .0 = [r, g, b, a];
            }
            self.mipmaps[i].image = Some(img);
        }
        Ok(())
    }
}
