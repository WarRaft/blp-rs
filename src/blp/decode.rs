use crate::{
    BlpError,
    blp::{Blp, Encoding, Frame, Version},
};
use image::RgbaImage;
use jpeg_decoder::{Decoder, PixelFormat};

impl Blp {
    /// Decodes one mip (a frame from [`Blp::parse`]) of the file `buf`.
    pub fn decode(&self, frame: &Frame, buf: &[u8]) -> Result<RgbaImage, BlpError> {
        if frame.length == 0 {
            let key = if self.version == Version::BLP0 { "blp.mip.external" } else { "blp.mip.empty" };
            return Err(BlpError::new(key)
                .with_arg("width", frame.width)
                .with_arg("height", frame.height));
        }
        self.decode_data(frame, frame.data(buf)?, buf)
    }

    /// Decodes a mip's bytes `data` held outside the file `buf`: BLP0
    /// keeps mip `i` of `name.blp` in `name.b0i` (`name.b1i` from 10 on).
    /// `frame` gives the mip's size.
    pub fn decode_data(&self, frame: &Frame, data: &[u8], buf: &[u8]) -> Result<RgbaImage, BlpError> {
        let (w, h) = (frame.width, frame.height);
        let rgba = match self.encoding()? {
            Encoding::Jpeg => return jpeg(self.header_data(buf)?, data, self.alpha_bits),
            Encoding::Palette => palette(self.header_data(buf)?, data, self.alpha_bits, pixels(w, h)?)?,
            Encoding::Bgra => bgra(data, pixels(w, h)?)?,
            kind => dxt(kind, data, w, h, self.alpha_bits)?,
        };
        Ok(RgbaImage::from_raw(w, h, rgba).expect("decoders fill the whole image"))
    }
}

/// Pixel count, refusing sizes that can't be held in memory.
fn pixels(w: u32, h: u32) -> Result<usize, BlpError> {
    (w as usize)
        .checked_mul(h as usize)
        .filter(|n| n.checked_mul(4).is_some())
        .ok_or_else(|| {
            BlpError::new("blp.size.too-large")
                .with_arg("width", w)
                .with_arg("height", h)
        })
}

fn truncated(key: &'static str, need: usize, have: usize) -> BlpError {
    BlpError::new(key)
        .with_arg("need", need)
        .with_arg("have", have)
}

/// A JPEG mip: the shared header followed by the mip's own bytes. The
/// channels are stored as CMYK holding B, G, R, A.
fn jpeg(header: &[u8], data: &[u8], alpha_bits: u32) -> Result<RgbaImage, BlpError> {
    let mut full = Vec::with_capacity(header.len() + data.len());
    full.extend_from_slice(header);
    full.extend_from_slice(data);

    let mut dec = Decoder::new(full.as_slice());
    let pixels = dec
        .decode()
        .map_err(|e| BlpError::from(e).ctx("blp.jpeg.decode"))?;
    let info = dec
        .info()
        .ok_or_else(|| BlpError::new("blp.jpeg.no-info"))?;
    let (w, h) = (info.width as u32, info.height as u32);
    let opaque = alpha_bits == 0;

    let rgba: Vec<u8> = match info.pixel_format {
        // jpeg-decoder inverts CMYK, as Adobe stores it; BLP doesn't.
        PixelFormat::CMYK32 => pixels
            .chunks_exact(4)
            .flat_map(|p| [255 - p[2], 255 - p[1], 255 - p[0], if opaque { 255 } else { 255 - p[3] }])
            .collect(),
        PixelFormat::RGB24 => pixels
            .chunks_exact(3)
            .flat_map(|p| [p[2], p[1], p[0], 255])
            .collect(),
        PixelFormat::L8 => pixels
            .iter()
            .flat_map(|&l| [l, l, l, 255])
            .collect(),
        PixelFormat::L16 => pixels
            .chunks_exact(2)
            .flat_map(|p| {
                let l = p[0]; // big-endian: the high byte
                [l, l, l, 255]
            })
            .collect(),
    };
    RgbaImage::from_raw(w, h, rgba).ok_or_else(|| {
        BlpError::new("blp.jpeg.size")
            .with_arg("width", w)
            .with_arg("height", h)
    })
}

/// A palette mip: one index per pixel, then the alpha packed at
/// `alpha_bits` per pixel, low bits first.
fn palette(palette: &[u8], data: &[u8], alpha_bits: u32, n: usize) -> Result<Vec<u8>, BlpError> {
    let alpha_len = match alpha_bits {
        0 => 0,
        1 => n.div_ceil(8),
        4 => n.div_ceil(2),
        8 => n,
        _ => return Err(BlpError::new("blp.alpha-bits.unsupported").with_arg("alpha_bits", alpha_bits)),
    };
    if data.len() < n + alpha_len {
        return Err(truncated("blp.palette.truncated", n + alpha_len, data.len()));
    }
    let (indices, alpha) = data.split_at(n);
    let mut out = Vec::with_capacity(n * 4);
    for (p, &i) in indices.iter().enumerate() {
        let c = &palette[i as usize * 4..][..3];
        let a = match alpha_bits {
            1 => ((alpha[p / 8] >> (p % 8)) & 1) * 255,
            4 => ((alpha[p / 2] >> ((p % 2) * 4)) & 0x0F) * 17,
            8 => alpha[p],
            _ => 255,
        };
        out.extend_from_slice(&[c[2], c[1], c[0], a]);
    }
    Ok(out)
}

/// A raw mip: BGRA.
fn bgra(data: &[u8], n: usize) -> Result<Vec<u8>, BlpError> {
    let data = data
        .get(..n * 4)
        .ok_or_else(|| truncated("blp.bgra.truncated", n * 4, data.len()))?;
    Ok(data
        .chunks_exact(4)
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect())
}

/// A DXT (S3TC) mip: 4×4 blocks, 8 bytes each for DXT1, 16 for DXT3/5.
fn dxt(kind: Encoding, data: &[u8], w: u32, h: u32, alpha_bits: u32) -> Result<Vec<u8>, BlpError> {
    let (w, h) = (w as usize, h as usize);
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let block_len = if kind == Encoding::Dxt1 { 8 } else { 16 };
    let need = bw * bh * block_len;
    if data.len() < need {
        return Err(truncated("blp.dxt.truncated", need, data.len()));
    }

    let mut out = vec![0u8; pixels(w as u32, h as u32)? * 4];
    for (b, block) in data[..need]
        .chunks_exact(block_len)
        .enumerate()
    {
        let (alpha, color) = block.split_at(block_len - 8);
        let mut texels = color_block(color, kind == Encoding::Dxt1);
        match kind {
            Encoding::Dxt3 => {
                for (t, texel) in texels.iter_mut().enumerate() {
                    texel[3] = ((alpha[t / 2] >> ((t % 2) * 4)) & 0x0F) * 17;
                }
            }
            Encoding::Dxt5 => {
                let a = alpha_block(alpha);
                for (texel, a) in texels.iter_mut().zip(a) {
                    texel[3] = a;
                }
            }
            _ if alpha_bits == 0 => texels
                .iter_mut()
                .for_each(|t| t[3] = 255),
            _ => {}
        }

        let (x0, y0) = ((b % bw) * 4, (b / bw) * 4);
        for (t, texel) in texels.iter().enumerate() {
            let (x, y) = (x0 + t % 4, y0 + t / 4);
            if x < w && y < h {
                out[(y * w + x) * 4..][..4].copy_from_slice(texel);
            }
        }
    }
    Ok(out)
}

/// The 16 texels of a color block: two RGB565 endpoints and 2-bit
/// indices. DXT1 with `c0 <= c1` has three colors and transparent black.
fn color_block(block: &[u8], dxt1: bool) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let (a, b) = (rgb565(c0), rgb565(c1));
    let mix = |wa: u16, wb: u16, d: u16| -> [u8; 4] {
        let ch = |i: usize| ((a[i] as u16 * wa + b[i] as u16 * wb) / d) as u8;
        [ch(0), ch(1), ch(2), 255]
    };
    let colors = if c0 > c1 || !dxt1 {
        [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], mix(2, 1, 3), mix(1, 2, 3)]
    } else {
        [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], mix(1, 1, 2), [0, 0, 0, 0]]
    };
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    std::array::from_fn(|t| colors[((indices >> (t * 2)) & 3) as usize])
}

/// The 16 alphas of a DXT5 alpha block: two endpoints and 3-bit indices.
fn alpha_block(block: &[u8]) -> [u8; 16] {
    let (a0, a1) = (block[0] as u16, block[1] as u16);
    let mut values = [0u8; 8];
    values[0] = a0 as u8;
    values[1] = a1 as u8;
    if a0 > a1 {
        for i in 1..7u16 {
            values[i as usize + 1] = (((7 - i) * a0 + i * a1) / 7) as u8;
        }
    } else {
        for i in 1..5u16 {
            values[i as usize + 1] = (((5 - i) * a0 + i * a1) / 5) as u8;
        }
        values[6] = 0;
        values[7] = 255;
    }
    let mut bits = [0u8; 8];
    bits[..6].copy_from_slice(&block[2..8]);
    let indices = u64::from_le_bytes(bits);
    std::array::from_fn(|t| values[((indices >> (t * 3)) & 7) as usize])
}

/// Expands RGB565 to 8 bits per channel.
fn rgb565(c: u16) -> [u8; 3] {
    let (r, g, b) = ((c >> 11) & 31, (c >> 5) & 63, c & 31);
    [((r << 3) | (r >> 2)) as u8, ((g << 2) | (g >> 4)) as u8, ((b << 3) | (b >> 2)) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_expands_to_full_range() {
        assert_eq!(rgb565(0xFFFF), [255, 255, 255]);
        assert_eq!(rgb565(0), [0, 0, 0]);
        assert_eq!(rgb565(0xF800), [255, 0, 0]);
    }

    #[test]
    fn dxt1_three_color_mode_has_transparent_black() {
        // c0 = black <= c1 = white: index 3 is transparent.
        let block = [0, 0, 0xFF, 0xFF, 0b1111_1111, 0, 0, 0];
        let t = color_block(&block, true);
        assert_eq!(t[0], [0, 0, 0, 0]);
        assert_eq!(t[4], [0, 0, 0, 255]);
    }

    #[test]
    fn dxt5_alpha_interpolates() {
        let block = [255, 0, 0b001, 0, 0, 0, 0, 0];
        let a = alpha_block(&block);
        assert_eq!(a[0], 0);
        assert_eq!(a[1], 255);
    }

    #[test]
    fn palette_alpha_is_packed_low_bits_first() {
        let mut pal = vec![0u8; 1024];
        pal[4..8].copy_from_slice(&[10, 20, 30, 0]); // entry 1: B G R
        let data = [1, 1, 0b0000_0010];
        let out = palette(&pal, &data, 1, 2).unwrap();
        assert_eq!(out, [30, 20, 10, 0, 30, 20, 10, 255]);
        assert!(palette(&pal, &data[..2], 1, 2).is_err());
    }
}
