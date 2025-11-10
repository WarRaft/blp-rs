use crate::_error::error::BlpError;
use crate::_mipmap::Mipmap;
use crate::_types::{TextureType, Version};

pub const MAX_MIPS: usize = 16;
pub const HEADER_SIZE: u64 = 156;

/// Checks if buffer is a BLP file by signature
fn is_blp_file(buf: &[u8]) -> bool {
    // BLP files start with "BLP1" or "BLP2" signature (or just "BLP" prefix)
    buf.len() >= 3 && &buf[..3] == b"BLP"
}

#[derive(Debug, Default)]
pub struct BlpImage {
    #[allow(dead_code)]
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
    //
    pub mipmaps: Vec<Mipmap>,
    pub holes: usize,
    pub header_offset: usize,
    pub header_length: usize,
}

/// Lightweight metadata for a mip level (no pixel materialization)
pub struct MipInfo {
    pub index: usize,
    pub width: u32,
    pub height: u32,
    pub offset: usize,
    pub length: usize,
}

/// Metadata summary for a BLP buffer. Does not allocate pixel images.
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

impl BlpImage {
    pub fn from_buf(buf: &[u8]) -> Result<Self, BlpError> {
        // Only BLP buffers are supported by the core BlpImage constructor now.
        if is_blp_file(buf) { Self::from_buf_blp(buf) } else { Err(BlpError::new("error-not-blp")) }
    }

    /// Create BLP from raw RGBA buffer.
    /// Buffer must be in RGBA format (4 bytes per pixel).
    /// Width and height must match the buffer size.
    pub fn from_rgba(rgba_buf: &[u8], width: u32, height: u32) -> Result<Self, BlpError> {
        Self::from_rgba_impl(rgba_buf, width, height)
    }

    /// Top-level decode entry.
    ///
    /// `mip_visible[i] == false` → skip decoding for mip `i`.
    /// Missing indices are treated as `true`.
    pub fn decode(&mut self, buf: &[u8], mip_visible: &[bool]) -> Result<(), BlpError> {
        // Decode BLP payloads according to texture type.
        match self.texture_type {
            TextureType::DIRECT => self.decode_direct(buf, mip_visible),
            TextureType::JPEG => self.decode_jpeg(buf, mip_visible),
        }
    }

    /// Inspect raw BLP bytes and return metadata without decoding pixel data.
    /// Returns an error if buffer is not a BLP or is truncated/corrupted.
    pub fn inspect_buf(buf: &[u8]) -> Result<BlpMeta, BlpError> {
        // We can reuse the internal parser that populates BlpImage fields without
        // materializing images: from_buf_blp is crate-visible and used here.
        let img = Self::from_buf_blp(buf)?;

        let mut mipinfos = Vec::with_capacity(img.mipmaps.len());
        for (i, m) in img.mipmaps.iter().enumerate() {
            mipinfos.push(MipInfo { index: i, width: m.width, height: m.height, offset: m.offset, length: m.length });
        }

        Ok(BlpMeta { version: img.version, texture_type: img.texture_type, compression: img.compression, alpha_bits: img.alpha_bits, alpha_type: img.alpha_type, has_mips: img.has_mips, width: img.width, height: img.height, extra: img.extra, has_mipmaps: img.has_mipmaps, mipmaps: mipinfos, holes: img.holes, header_offset: img.header_offset, header_length: img.header_length })
    }

    /// Return a slice pointing at the shared JPEG header (for JPEG texture type).
    pub fn shared_jpeg_header<'a>(&self, buf: &'a [u8]) -> Option<&'a [u8]> {
        if let crate::_types::TextureType::JPEG = self.texture_type {
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
        use crate::_types::TextureType;
        if let TextureType::DIRECT = self.texture_type {
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
        let mut img = Self::from_rgba(rgba_buf, width, height)?;
        let ctx = img.encode_blp(quality, mip_visible)?;
        Ok(ctx.bytes)
    }
}
