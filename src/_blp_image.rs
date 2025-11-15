// Backwards-compatibility shim. BLP header parsing, metadata and helpers
// were moved to `crate::blp::blp`. Keep this minimal wrapper to avoid
// duplicating business logic.

pub use crate::blp::blp::HEADER_SIZE;
pub use crate::blp::MAX_MIPS;
pub use crate::blp::Blp as BlpImage;
pub use crate::blp::Frame as Mipmap;
pub use crate::blp::Blp as BlpMeta; // legacy name; prefer crate::blp::Blp

pub use crate::blp::{parse_header, from_rgba as from_rgba_impl, shared_jpeg_header as shared_jpeg_header_impl, mip_raw as mip_raw_impl};

impl BlpImage {
    pub fn from_buf(buf: &[u8]) -> Result<Self, BlpError> {
        // Only BLP buffers are supported by the core BlpImage constructor now.
        if is_blp_file(buf) {
            let b = crate::blp::parse_header(buf)?;
            Ok(BlpImage {
                version: b.version,
                texture_type: b.texture_type,
                compression: b.compression,
                alpha_bits: b.alpha_bits,
                alpha_type: b.alpha_type,
                has_mips: b.has_mips,
                width: b.width,
                height: b.height,
                extra: b.extra,
                has_mipmaps: b.has_mipmaps,
                mipmaps: b.frames,
                holes: b.holes,
                header_offset: b.header.offset,
                header_length: b.header.length,
            })
        } else {
            Err(BlpError::new("error-not-blp"))
        }
    }

    /// Create BLP from raw RGBA buffer.
    /// Buffer must be in RGBA format (4 bytes per pixel).
    /// Width and height must match the buffer size.
    pub fn from_rgba(rgba_buf: &[u8], width: u32, height: u32) -> Result<Self, BlpError> {
        // Convert canonical Blp header into legacy BlpImage for backward compat
        let blp = crate::blp::from_rgba(rgba_buf, width, height)?;
        Ok(BlpImage {
            version: blp.version,
            texture_type: blp.texture_type,
            compression: blp.compression,
            alpha_bits: blp.alpha_bits,
            alpha_type: blp.alpha_type,
            has_mips: blp.has_mips,
            width: blp.width,
            height: blp.height,
            extra: blp.extra,
            has_mipmaps: blp.has_mipmaps,
            mipmaps: blp.frames,
            holes: blp.holes,
            header_offset: blp.header.offset,
            header_length: blp.header.length,
        })
    }

    /// Top-level decode entry.
    ///
    /// `mip_visible[i] == false` → skip decoding for mip `i`.
    /// Missing indices are treated as `true`.
    pub fn decode(&mut self, buf: &[u8], mip_visible: &[bool]) -> Result<(), BlpError> {
        // Decode BLP payloads according to texture type.
        match self.texture_type {
            TextureType::PALETTE => {
                // Decode directly but ignore produced images for legacy BlpImage.
                let _ = crate::_decode::decode_direct_to_mipmaps(&crate::blp::parse_header(buf)?, buf, mip_visible)?;
                Ok(())
            }
            TextureType::JPEG => {
                let _ = crate::_decode::decode_jpeg_to_mipmaps(&crate::blp::parse_header(buf)?, buf, mip_visible)?;
                Ok(())
            }
        }
    }

    /// Inspect raw BLP bytes and return metadata without decoding pixel data.
    /// Returns an error if buffer is not a BLP or is truncated/corrupted.
    pub fn inspect_buf(buf: &[u8]) -> Result<BlpMeta, BlpError> {
        // We can reuse the canonical Blp header parser.
        let img = crate::blp::parse_header(buf)?;

            let mut mipinfos = Vec::with_capacity(img.frames.len());
        for (i, m) in img.frames.iter().enumerate() {
            mipinfos.push(MipInfo { index: i, width: m.width, height: m.height, offset: m.offset, length: m.length });
        }

        Ok(BlpMeta { version: img.version, texture_type: img.texture_type, compression: img.compression, alpha_bits: img.alpha_bits, alpha_type: img.alpha_type, has_mips: img.has_mips, width: img.width, height: img.height, extra: img.extra, has_mipmaps: img.has_mipmaps, mipmaps: mipinfos, holes: img.holes, header_offset: img.header.offset, header_length: img.header.length })
    }

    /// Return a slice pointing at the shared JPEG header (for JPEG texture type).
    pub fn shared_jpeg_header<'a>(&self, buf: &'a [u8]) -> Option<&'a [u8]> {
        if let crate::blp::TextureType::JPEG = self.texture_type {
            let off = self.header_offset;
            let len = self.header_length;
            if off.checked_add(len).is_some() && off + len <= buf.len() {
                return Some(&buf[off..off + len]);
            }
        }
        None
    }

    /// For JPEG-based BLPs, return the raw tail bytes for a mip index (the part
    /// that was concatenated to the shared header). For DIRECT textures returns
    /// the raw payload for the mip (palette + pixels depending on format).
    pub fn mip_raw<'a>(&self, buf: &'a [u8], mip_index: usize) -> Option<&'a [u8]> {
        if mip_index >= self.mipmaps.len() {
            return None;
        }
        let m = &self.mipmaps[mip_index];
        if m.length == 0 {
            return None;
        }
        let off = m.offset;
        let len = m.length;
        if off.checked_add(len).is_none() || off + len > buf.len() {
            return None;
        }
        Some(&buf[off..off + len])
    }

    /// For DIRECT (paletted) textures, return the palette bytes if present.
    /// Palette layout: sequence of 256 RGBA entries (1024 bytes) starting at header_offset.
    pub fn palette_bytes<'a>(&self, buf: &'a [u8]) -> Option<&'a [u8]> {
        if let crate::blp::TextureType::PALETTE = self.texture_type {
            let off = self.header_offset;
            let len = self.header_length;
            if off.checked_add(len).is_some() && off + len <= buf.len() {
                return Some(&buf[off..off + len]);
            }
        }
        None
    }

    /// High-level helper: encode an RGBA raw buffer into BLP bytes with given quality
    /// and mip visibility mask. This wraps `from_rgba` -> `encode_blp` and returns
    /// the produced bytes.
    pub fn encode_rgba_to_blp(rgba_buf: &[u8], width: u32, height: u32, quality: u8, mip_visible: &[bool]) -> Result<Vec<u8>, BlpError> {
        // Use canonical Blp builder then call the encoder on the canonical type
        let mut img = crate::blp::from_rgba(rgba_buf, width, height)?;
        // frame images for encoder: base image at index 0
        use image::ImageBuffer;
        let base = ImageBuffer::from_raw(width, height, rgba_buf.to_vec()).ok_or_else(|| BlpError::new("error-rgba-image-creation"))?;
        let mut frame_images: Vec<Option<image::RgbaImage>> = vec![None; img.frames.len()];
        frame_images[0] = Some(base);
        let ctx = img.encode_blp(quality, mip_visible, &frame_images)?;
        Ok(ctx.bytes)
    }
}
