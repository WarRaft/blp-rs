use crate::blp::{Blp, Frame, TextureType};
use crate::error::error::BlpError;
use byteorder::{LittleEndian, ReadBytesExt};
use image::RgbaImage;
use jpeg_decoder::{Decoder, PixelFormat};
use std::io::Cursor;

/// Decode a single JPEG-based BLP frame.
pub(crate) fn decode_jpeg_frame(img: &Blp, frame: &Frame, buf: &[u8]) -> Result<RgbaImage, BlpError> {
    // --- Validate header range and slice it out ---
    let h_off = img.header.offset;
    let h_len = img.header.length;
    if h_off.checked_add(h_len).is_none() || h_off + h_len > buf.len() {
        return Err(BlpError::new("jpeg.header.oob"));
    }
    let header_bytes = &buf[h_off..h_off + h_len];

    // If alpha_bits == 0 we force opaque alpha channel when reconstructing RGBA.
    let force_opaque = img.alpha_bits == 0;

    let off = frame.offset;
    let len = frame.length;
    if len == 0 {
        return Err(BlpError::new("jpeg.frame.empty"));
    }
    if off.checked_add(len).is_none() || off + len > buf.len() {
        return Err(BlpError::new("jpeg.frame.oob"));
    }

    let tail = &buf[off..off + len];
    let mut full = Vec::with_capacity(header_bytes.len() + tail.len());
    full.extend_from_slice(header_bytes);
    full.extend_from_slice(tail);

    let mut dec = Decoder::new(Cursor::new(&full));
    dec.read_info()
        .map_err(|e| BlpError::from(e).with_arg("phase", "read_info"))?;
    let info = dec
        .info()
        .ok_or_else(|| BlpError::new("jpeg.meta.missing"))?;
    let (w, h) = (info.width as u32, info.height as u32);
    let pixels = dec
        .decode()
        .map_err(|e| BlpError::from(e).with_arg("phase", "decode"))?;

    let mut imgbuf = RgbaImage::new(w, h);
    match info.pixel_format {
        PixelFormat::CMYK32 => {
            for (p, px) in imgbuf.pixels_mut().enumerate() {
                let idx = p * 4;
                let c = pixels[idx];
                let m = pixels[idx + 1];
                let y = pixels[idx + 2];
                let k = pixels[idx + 3];
                let a = if force_opaque { 255 } else { 255u8.saturating_sub(k) };
                *px = image::Rgba([255u8.saturating_sub(y), 255u8.saturating_sub(m), 255u8.saturating_sub(c), a]);
            }
        }
        PixelFormat::RGB24 => {
            for (p, px) in imgbuf.pixels_mut().enumerate() {
                let idx = p * 3;
                *px = image::Rgba([pixels[idx + 2], pixels[idx + 1], pixels[idx + 0], 255]);
            }
        }
        PixelFormat::L8 => {
            for (p, px) in imgbuf.pixels_mut().enumerate() {
                let l = pixels[p];
                *px = image::Rgba([l, l, l, 255]);
            }
        }
        PixelFormat::L16 => {
            for (chunk, px) in pixels
                .chunks_exact(2)
                .zip(imgbuf.pixels_mut())
            {
                let l16 = u16::from_be_bytes([chunk[0], chunk[1]]);
                let l8 = (l16 / 257) as u8;
                *px = image::Rgba([l8, l8, l8, 255]);
            }
        }
    }

    Ok(imgbuf)
}

/// Decode a single PALETTE BLP frame.
pub(crate) fn decode_palette_frame(img: &Blp, frame: &Frame, buf: &[u8]) -> Result<RgbaImage, BlpError> {
    use std::io::Read;

    if img.header.offset + img.header.length > buf.len() {
        return Err(BlpError::new("direct.header.oob"));
    }

    let mut cur = Cursor::new(&buf[..]);
    cur.set_position(img.header.offset as u64);
    let mut palette = [[0u8; 3]; 256];
    for i in 0..256 {
        let color = cur.read_u32::<LittleEndian>()?;
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;
        palette[i] = [r, g, b];
    }

    let alpha_bits = img.alpha_bits;
    let off = frame.offset;
    let len = frame.length;

    if len == 0 {
        return Err(BlpError::new("direct.frame.empty"));
    }
    if off.checked_add(len).is_none() || off + len > buf.len() {
        return Err(BlpError::new("direct.frame.oob"));
    }

    cur.set_position(off as u64);
    let (w, h) = (frame.width, frame.height);
    let pixel_count = (w as usize) * (h as usize);

    let mut indices = vec![0u8; pixel_count];
    cur.read_exact(&mut indices)
        .map_err(|_| BlpError::new("direct.indices.truncated"))?;

    let alpha_bytes = match alpha_bits {
        0 => 0,
        1 => (pixel_count + 7) / 8,
        4 => (pixel_count + 1) / 2,
        8 => pixel_count,
        _ => return Err(BlpError::new("blp.version.invalid").with_arg("msg", "unsupported alpha bits")),
    };

    let mut alpha_raw = vec![0u8; alpha_bytes];
    if alpha_bytes > 0 {
        cur.read_exact(&mut alpha_raw)
            .map_err(|_| BlpError::new("direct.alpha.truncated"))?;
    }

    let mut out_img = RgbaImage::new(w, h);
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
        out_img
            .get_pixel_mut((p as u32) % w, (p as u32) / w)
            .0 = [r, g, b, a];
    }

    Ok(out_img)
}

/// Open and decode all mipmaps from a BLP file (with visibility filter).
///
/// Returns a vector of Option<RgbaImage>, where None indicates
/// the mipmap was not decoded (either missing or mip_visible was false).
pub(crate) fn open_mipmaps_filtered(blp: &Blp, frames: &[Frame], buf: &[u8], mip_visible: &[bool]) -> Result<Vec<Option<RgbaImage>>, BlpError> {
    let mut out = Vec::with_capacity(frames.len());
    for (i, frame) in frames.iter().enumerate() {
        let visible = mip_visible
            .get(i)
            .copied()
            .unwrap_or(true);
        if !visible || frame.length == 0 {
            out.push(None);
            continue;
        }
        let result = match blp.texture_type {
            TextureType::JPEG => decode_jpeg_frame(blp, frame, buf),
            TextureType::PALETTE => decode_palette_frame(blp, frame, buf),
        };
        match result {
            Ok(img) => out.push(Some(img)),
            Err(_) => out.push(None),
        }
    }
    Ok(out)
}

/// Open and decode all mipmaps from a BLP buffer.
///
/// This is a convenience function that returns all non-empty mipmaps
/// as RgbaImage instances (skipping empty frames).
pub(crate) fn open_mipmaps(buf: &[u8]) -> Result<Vec<RgbaImage>, BlpError> {
    use crate::blp::MAX_MIPS;
    use crate::blp::parse::parse_header;

    let (img, frames) = parse_header(buf)?;
    let mut out = Vec::new();
    for frame in frames.iter().take(MAX_MIPS) {
        if frame.length == 0 {
            continue;
        }
        let rgba = match img.texture_type {
            TextureType::JPEG => decode_jpeg_frame(&img, frame, buf)?,
            TextureType::PALETTE => decode_palette_frame(&img, frame, buf)?,
        };
        out.push(rgba);
    }
    Ok(out)
}

impl Blp {
    /// Decode an external image (PNG/JPG/PSD/etc.) into power-of-two mip images
    /// and fill `frames[*]` dimensions accordingly.
    pub fn decode_image(&self, frames: &mut [Frame], buf: &[u8], mip_visible: &[bool]) -> Result<Vec<Option<RgbaImage>>, BlpError> {
        // --- Decode source into RGBA8 ---
        use crate::traits::{FormatDetector, ImageDecoder};

        let src = if Blp::detect(buf) {
            Blp::into_dynamic(buf)?
        } else if crate::psd::PsdImage::detect(buf) {
            crate::psd::PsdImage::into_dynamic(buf)?
        } else {
            image::load_from_memory(buf).map_err(|_| BlpError::new("error-image-load"))?
        };

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
        let resized = image::imageops::resize(&src, rw, rh, image::imageops::FilterType::Lanczos3);

        // --- (2) center-crop to exactly (tw, th) ---
        let cx = ((rw.saturating_sub(tw)) / 2).min(rw.saturating_sub(tw));
        let cy = ((rh.saturating_sub(th)) / 2).min(rh.saturating_sub(th));
        let base = image::imageops::crop_imm(&resized, cx, cy, tw, th).to_image();

        // --- (3) build mip chain, honoring `mip_visible` ---
        let mut prev = base;
        let (mut w, mut h) = (tw, th);

        let mut out: Vec<Option<RgbaImage>> = Vec::with_capacity(frames.len());
        for i in 0..frames.len() {
            // Record dimensions for this mip (even if we skip pixels).
            frames[i].width = w;
            frames[i].height = h;

            // Visibility gate: missing entry → treated as `true`.
            let visible = mip_visible
                .get(i)
                .copied()
                .unwrap_or(true);
            if visible {
                out.push(Some(prev.clone()));
            } else {
                out.push(None);
            }

            // Stop when we reached 1×1.
            if w == 1 && h == 1 {
                break;
            }

            // Next mip level dims: halve each dimension, clamp to ≥1.
            let next_w = (w / 2).max(1);
            let next_h = (h / 2).max(1);

            // Downscale current level into the next.
            let next_img = image::imageops::resize(&prev, next_w, next_h, image::imageops::FilterType::Lanczos3);

            prev = next_img;
            w = next_w;
            h = next_h;
        }

        while out.len() < frames.len() {
            out.push(None);
        }
        Ok(out)
    }
}

impl crate::traits::ImageDecoder for Blp {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        use crate::blp::parse::parse_header;

        // Decode only the first mipmap
        let (blp, frames) = parse_header(buf)?;
        if frames.is_empty() {
            return Err(BlpError::new("blp.no-frames"));
        }
        let frame = &frames[0];
        let img = match blp.texture_type {
            TextureType::JPEG => decode_jpeg_frame(&blp, frame, buf)?,
            TextureType::PALETTE => decode_palette_frame(&blp, frame, buf)?,
        };
        Ok(image::DynamicImage::ImageRgba8(img))
    }
}
