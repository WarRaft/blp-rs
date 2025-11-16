use num_enum::TryFromPrimitive;

pub const MAX_MIPS: usize = 16;
pub const HEADER_SIZE: u64 = 156;

#[derive(Debug, Clone)]
pub struct Blp {
    pub version: Version,
    pub texture_type: TextureType,
    pub compression: u8,
    pub alpha_bits: u32,
    pub alpha_type: u8,
    pub has_mips: u8,
    pub width: u32,
    pub height: u32,
    pub extra: u32,       // meaningful only if version <= BLP1
    pub has_mipmaps: u32, // meaningful only if version <= BLP1 or >= BLP2
    pub holes: usize,
    pub header: Frame,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum Version {
    BLP0 = 0x424C5030, // "BLP0"
    // FormatDetector implementation for Blp moved to the bottom of the file.
    #[default]
    BLP1 = 0x424C5031, // "BLP1"
    BLP2 = 0x424C5032, // "BLP2"
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum TextureType {
    #[default]
    JPEG = 0,
    PALETTE = 1,
}

#[derive(Debug, Default, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub offset: usize,
    pub length: usize,
    // No image data stored here; only metadata
}

use crate::error::error::BlpError;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use image::RgbaImage;
use std::io::Cursor;

/// Parse header-only information from a BLP buffer and return a `Blp` struct.
/// This reuses existing low-level parsing and returns a header-only `Blp`.
pub fn parse_header(buf: &[u8]) -> Result<(Blp, Vec<Frame>), BlpError> {
    let mut cursor = Cursor::new(buf);

    let version_raw = cursor.read_u32::<BigEndian>()?;
    let version = Version::try_from(version_raw)?;

    let texture_type_raw = cursor.read_u32::<LittleEndian>()?;
    let texture_type = TextureType::try_from(texture_type_raw)?;

    let (compression, alpha_bits, alpha_type, has_mips) = if version >= Version::BLP2 {
        (cursor.read_u8()?, cursor.read_u8()? as u32, cursor.read_u8()?, cursor.read_u8()?)
    } else {
        (0u8, cursor.read_u32::<LittleEndian>()?, 0u8, 0u8)
    };

    let width = cursor.read_u32::<LittleEndian>()?;
    let height = cursor.read_u32::<LittleEndian>()?;

    let (extra, has_mipmaps) = if version <= Version::BLP1 {
        (cursor.read_u32::<LittleEndian>()?, cursor.read_u32::<LittleEndian>()?)
    } else {
        (0u32, has_mips as u32)
    };

    // read offsets/lengths
    let mut frames_arr: [Frame; MAX_MIPS] = std::array::from_fn(|_| Frame::default());
    let (mut w, mut h) = (width, height);

    let mi = (32 - width.max(height).leading_zeros()) as usize;

    if version >= Version::BLP1 {
        for i in 0..MAX_MIPS {
            frames_arr[i].offset = cursor.read_u32::<LittleEndian>()? as usize;
        }
        for i in 0..MAX_MIPS {
            frames_arr[i].length = cursor.read_u32::<LittleEndian>()? as usize;
            if i < mi {
                frames_arr[i].width = w;
                w = (w / 2).max(1);

                frames_arr[i].height = h;
                h = (h / 2).max(1);
            }
        }
    }

    // header offset / length
    let (header_offset, header_length) = match texture_type {
        TextureType::JPEG => {
            let base = HEADER_SIZE as usize;
            if buf.len() < base + 4 {
                return Err(BlpError::new("truncated: cannot read JPEG header size"));
            }
            let mut c = Cursor::new(&buf[base..]);
            let hdr_len = c.read_u32::<LittleEndian>()? as usize;
            let hdr_off = base + 4;
            if buf.len() < hdr_off + hdr_len {
                return Err(BlpError::new("truncated: JPEG header out of bounds"));
            }
            (hdr_off, hdr_len)
        }
        TextureType::PALETTE => (HEADER_SIZE as usize, 256 * 4),
    };

    // compute holes
    let mut ranges = Vec::new();
    for i in 0..MAX_MIPS {
        let off = frames_arr[i].offset;
        let len = frames_arr[i].length;
        if len == 0 {
            continue;
        }
        if let Some(end) = off.checked_add(len) {
            if end <= buf.len() {
                ranges.push((off, end));
            }
        }
    }
    ranges.sort_by_key(|r| r.0);

    let mut prev_end = header_offset + header_length;
    let mut holes = 0usize;
    for (start, end) in &ranges {
        if *start >= prev_end {
            holes += start - prev_end;
        }
        if *end > prev_end {
            prev_end = *end;
        }
    }
    if buf.len() > prev_end {
        holes += buf.len() - prev_end;
    }

    let frames = frames_arr
        .into_iter()
        .map(|f| Frame { width: f.width, height: f.height, offset: f.offset, length: f.length })
        .collect();
    let header = Frame { width: 0, height: 0, offset: header_offset, length: header_length };

    let blp = Blp { version, texture_type, compression, alpha_bits, alpha_type, has_mips, width, height, extra, has_mipmaps, holes, header };
    Ok((blp, frames))
}

/// Wrap `crate::blp::from_rgba` decode helper as part of blp module.
pub fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Result<(Blp, Vec<Frame>), BlpError> {
    if width == 0 || height == 0 {
        return Err(BlpError::new("error-image-empty")
            .with_arg("width", width)
            .with_arg("height", height));
    }
    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        return Err(BlpError::new("error-rgba-buffer-size")
            .with_arg("expected", expected)
            .with_arg("actual", rgba.len()));
    }

    // build RgbaImage (not stored in Blp directly here)
    let _base_img = RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or_else(|| BlpError::new("error-rgba-image-creation"))?;

    // build power-of-two mip chain
    let levels = (32 - width.max(height).leading_zeros()) as usize;
    let mut frames: Vec<Frame> = Vec::with_capacity(MAX_MIPS);
    let (mut w, mut h) = (width, height);
    for i in 0..MAX_MIPS {
        if i < levels {
            frames.push(Frame { width: w, height: h, offset: 0, length: 0 });
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        } else {
            frames.push(Frame::default());
        }
    }

    let blp = Blp { version: Version::BLP1, texture_type: TextureType::JPEG, compression: 0, alpha_bits: 0, alpha_type: 0, has_mips: 0, width, height, extra: 0, has_mipmaps: 0, holes: 0, header: Frame::default() };
    Ok((blp, frames))
}

/// Return header data (JPEG header for JPEG type, palette for PALETTE type).
/// 
/// For JPEG textures: returns shared JPEG header.
/// For PALETTE textures: returns palette bytes (256 RGBA entries, 1024 bytes total).
pub fn header_data(buf: &[u8]) -> Option<&[u8]> {
    if let Ok((h, _frames)) = parse_header(buf) {
        let off = h.header.offset;
        let len = h.header.length;
        if off.checked_add(len).is_some() && off + len <= buf.len() {
            return Some(&buf[off..off + len]);
        }
    }
    None
}

/// Deprecated: Use `header_data` instead.
#[deprecated(since = "1.1.0", note = "Use `header_data` instead")]
pub fn shared_jpeg_header(buf: &[u8]) -> Option<&[u8]> {
    header_data(buf)
}

/// Deprecated: Use `header_data` instead.
#[deprecated(since = "1.1.0", note = "Use `header_data` instead")]
pub fn palette_bytes(buf: &[u8]) -> Option<&[u8]> {
    header_data(buf)
}

/// Return the raw payload for a given mip index (no decoding).
pub fn mip_raw(buf: &[u8], mip_index: usize) -> Option<&[u8]> {
    if let Ok((_h, frames)) = parse_header(buf) {
        if mip_index >= frames.len() {
            return None;
        }
        let f = &frames[mip_index];
        if f.length == 0 {
            return None;
        }
        if f.offset.checked_add(f.length).is_none() || f.offset + f.length > buf.len() {
            return None;
        }
        return Some(&buf[f.offset..f.offset + f.length]);
    }
    None
}

/// Lightweight metadata for a mip level (no pixel materialization)
#[derive(Debug, Clone)]
pub struct MipInfo {
    pub index: usize,
    pub width: u32,
    pub height: u32,
    pub offset: usize,
    pub length: usize,
}

/// Metadata summary for a BLP buffer. Does not allocate pixel images.
#[derive(Debug, Clone)]
pub struct BlpMeta {
    pub version: Version,
    pub texture_type: TextureType,
    pub compression: u8,
    pub alpha_bits: u32,
    pub alpha_type: u8,
    pub has_mips: u8,
    pub width: u32,
    pub height: u32,
    pub extra: u32,
    pub has_mipmaps: u32,
    pub mipmaps: Vec<MipInfo>,
    pub holes: usize,
    pub header_offset: usize,
    pub header_length: usize,
}

/// Inspect raw BLP bytes and return metadata without decoding pixel data.
/// Returns an error if buffer is not a BLP or is truncated/corrupted.
pub fn inspect_buf(buf: &[u8]) -> Result<BlpMeta, BlpError> {
    let (h, frames) = parse_header(buf)?;

    let mut mipinfos = Vec::with_capacity(frames.len());
    for (i, f) in frames.iter().enumerate() {
        mipinfos.push(MipInfo { index: i, width: f.width, height: f.height, offset: f.offset, length: f.length });
    }

    Ok(BlpMeta { version: h.version, texture_type: h.texture_type, compression: h.compression, alpha_bits: h.alpha_bits, alpha_type: h.alpha_type, has_mips: h.has_mips, width: h.width, height: h.height, extra: h.extra, has_mipmaps: h.has_mipmaps, mipmaps: mipinfos, holes: h.holes, header_offset: h.header.offset, header_length: h.header.length })
}

/// Inspect image bytes and return its declared pixel dimensions without full materialization.
pub fn inspect_image_dimensions(buf: &[u8]) -> Result<(u32, u32), BlpError> {
    let (h, _frames) = parse_header(buf)?;
    Ok((h.width, h.height))
}

/// For BLP files return all mipmaps as owned `RgbaImage`s.
pub fn open_mipmaps(buf: &[u8]) -> Result<Vec<RgbaImage>, BlpError> {
    let (img, frames) = parse_header(buf)?;
    let mut out = Vec::new();
    for frame in frames.iter().take(MAX_MIPS) {
        if frame.length == 0 {
            continue;
        }
        let rgba = match img.texture_type {
            TextureType::JPEG => crate::blp::decode::decode_jpeg_frame(&img, frame, buf)?,
            TextureType::PALETTE => crate::blp::decode::decode_palette_frame(&img, frame, buf)?,
        };
        out.push(rgba);
    }
    Ok(out)
}

impl Blp {
    /// Decode the BLP payload into frame images according to texture type.
    pub fn decode(&self, frames: &[Frame], buf: &[u8], mip_visible: &[bool]) -> Result<Vec<Option<RgbaImage>>, BlpError> {
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
            let result = match self.texture_type {
                TextureType::JPEG => crate::blp::decode::decode_jpeg_frame(self, frame, buf),
                TextureType::PALETTE => crate::blp::decode::decode_palette_frame(self, frame, buf),
            };
            match result {
                Ok(img) => out.push(Some(img)),
                Err(_) => out.push(None),
            }
        }
        Ok(out)
    }
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

    /// Encode a DynamicImage to BLP format with mipmaps.
    /// 
    /// This static method handles the complete BLP encoding pipeline:
    /// 1. Converts source image to RGBA
    /// 2. Scales to power-of-two dimensions (upscaling if needed)
    /// 3. Generates mipmaps according to mip_visible mask
    /// 4. Encodes to BLP with specified quality
    pub fn encode_from_image(
        source: &image::DynamicImage,
        quality: u8,
        mip_visible: &[bool],
    ) -> Result<Vec<u8>, BlpError> {
        /// Round up to the nearest power of two.
        fn next_pow2(v: u32) -> u32 {
            if v == 0 { return 1; }
            let mut n = v - 1;
            n |= n >> 1;
            n |= n >> 2;
            n |= n >> 4;
            n |= n >> 8;
            n |= n >> 16;
            n + 1
        }

        // Convert to RGBA
        let rgba = source.to_rgba8();
        let (src_w, src_h) = rgba.dimensions();
        
        // Calculate target power-of-two dimensions
        let target_w = next_pow2(src_w);
        let target_h = next_pow2(src_h);
        
        // Resize to power-of-two if needed
        let base_img = if src_w != target_w || src_h != target_h {
            image::imageops::resize(
                &rgba,
                target_w,
                target_h,
                image::imageops::FilterType::Lanczos3
            )
        } else {
            rgba
        };
        
        // Create Blp header structure
        let (blp, frames) = from_rgba(
            base_img.as_raw(),
            target_w,
            target_h
        )?;
        
        // Generate mipmaps
        let mut frame_images: Vec<Option<RgbaImage>> = vec![None; frames.len()];
        frame_images[0] = Some(base_img.clone());
        
        let mut prev = base_img;
        let mut w = target_w;
        let mut h = target_h;
        
        for i in 1..frames.len() {
            if !mip_visible.get(i).copied().unwrap_or(true) {
                break;
            }
            
            let next_w = (w / 2).max(1);
            let next_h = (h / 2).max(1);
            
            let next_img = image::imageops::resize(
                &prev,
                next_w,
                next_h,
                image::imageops::FilterType::Lanczos3
            );
            
            frame_images[i] = Some(next_img.clone());
            prev = next_img;
            w = next_w;
            h = next_h;
            
            if w == 1 && h == 1 {
                break;
            }
        }
        
        // Encode to BLP
        let ctx = blp.encode_blp(quality, mip_visible, &frames, &frame_images)?;
        Ok(ctx.bytes)
    }
}

impl crate::traits::FormatDetector for Blp {
    fn detect(buf: &[u8]) -> bool {
        buf.len() >= 3 && &buf[0..3] == b"BLP"
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        parse_header(buf)
    }
}

impl crate::traits::ImageDecoder for Blp {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        // Decode only the first mipmap
        let (blp, frames) = parse_header(buf)?;
        if frames.is_empty() {
            return Err(BlpError::new("blp.no-frames"));
        }
        let frame = &frames[0];
        let img = match blp.texture_type {
            TextureType::JPEG => crate::blp::decode::decode_jpeg_frame(&blp, frame, buf)?,
            TextureType::PALETTE => crate::blp::decode::decode_palette_frame(&blp, frame, buf)?,
        };
        Ok(image::DynamicImage::ImageRgba8(img))
    }
}
