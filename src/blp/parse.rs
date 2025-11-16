use crate::blp::{Blp, Frame, TextureType, Version, HEADER_SIZE, MAX_MIPS};
use crate::error::error::BlpError;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

/// Parse a BLP header from a buffer.
///
/// Returns a tuple of (Blp, Vec<Frame>) containing the header information
/// and frame metadata for all mipmaps.
pub(crate) fn parse_header(buf: &[u8]) -> Result<(Blp, Vec<Frame>), BlpError> {
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

/// Returns the header data for this BLP.
///
/// For JPEG textures, returns the shared JPEG header.
/// For palette textures, returns the 256-color RGBA palette (1024 bytes).
pub(crate) fn header_data_with_blp<'a>(blp: &Blp, buf: &'a [u8]) -> Result<&'a [u8], BlpError> {
    let off = blp.header.offset;
    let len = blp.header.length;
    if off.checked_add(len).is_none() || (off + len) > buf.len() {
        return Err(BlpError::new("blp.header.oob"));
    }
    Ok(&buf[off..off + len])
}

/// Returns the header data for a BLP buffer.
///
/// This is a convenience function that parses the header first.
/// For JPEG textures: returns shared JPEG header.
/// For PALETTE textures: returns palette bytes (256 RGBA entries, 1024 bytes total).
pub(crate) fn header_data(buf: &[u8]) -> Option<&[u8]> {
    if let Ok((h, _frames)) = parse_header(buf) {
        let off = h.header.offset;
        let len = h.header.length;
        if off.checked_add(len).is_some() && off + len <= buf.len() {
            return Some(&buf[off..off + len]);
        }
    }
    None
}

/// Returns the raw (encoded) mipmap data without decoding.
///
/// For JPEG textures, this is the JPEG frame data (without the shared header).
/// For palette textures, this is the indexed pixel data.
pub(crate) fn mip_raw_with_frame<'a>(frame: &Frame, buf: &'a [u8]) -> Result<&'a [u8], BlpError> {
    let off = frame.offset;
    let len = frame.length;
    if len == 0 {
        return Ok(&[]);
    }
    if off.checked_add(len).is_none() || (off + len) > buf.len() {
        return Err(BlpError::new("blp.mip.oob"));
    }
    Ok(&buf[off..off + len])
}

/// Return the raw payload for a given mip index (no decoding).
pub(crate) fn mip_raw(buf: &[u8], mip_index: usize) -> Option<&[u8]> {
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
