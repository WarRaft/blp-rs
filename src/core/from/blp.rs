use crate::core::image::{HEADER_SIZE, ImageBlp, MAX_MIPS};
use crate::core::mipmap::Mipmap;
use crate::core::types::{SourceKind, TextureType, Version};
use crate::error::error::BlpError;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

impl ImageBlp {
    pub(crate) fn from_buf_blp(buf: &[u8]) -> Result<Self, BlpError> {
        let mut cursor = Cursor::new(buf);

        let version_raw = cursor.read_u32::<BigEndian>()?;
        let version = Version::try_from(version_raw)?;

        let texture_type_raw = cursor.read_u32::<LittleEndian>()?;
        let texture_type = TextureType::try_from(texture_type_raw)?;

        let (compression, alpha_bits, alpha_type, has_mips) = if version >= Version::BLP2 {
            (
                cursor.read_u8()?,        // compression
                cursor.read_u8()? as u32, // alpha_bits
                cursor.read_u8()?,        // alpha_type
                cursor.read_u8()?,        // has_mips flag
            )
        } else {
            (
                0u8,
                cursor.read_u32::<LittleEndian>()?, // alpha_bits
                0u8,
                0u8,
            )
        };

        let width = cursor.read_u32::<LittleEndian>()?;
        let height = cursor.read_u32::<LittleEndian>()?;

        let (extra, has_mipmaps) = if version <= Version::BLP1 {
            (cursor.read_u32::<LittleEndian>()?, cursor.read_u32::<LittleEndian>()?)
        } else {
            (0u32, has_mips as u32)
        };

        // --- читаем таблицы смещений/длин
        let mut mipmaps: [Mipmap; MAX_MIPS] = std::array::from_fn(|_| Mipmap::default());
        let (mut w, mut h) = (width, height);

        let mi = (32 - width.max(height).leading_zeros()) as usize;

        if version >= Version::BLP1 {
            for i in 0..MAX_MIPS {
                mipmaps[i].offset = cursor.read_u32::<LittleEndian>()? as usize;
            }
            for i in 0..MAX_MIPS {
                mipmaps[i].length = cursor.read_u32::<LittleEndian>()? as usize;
                if i < mi {
                    mipmaps[i].width = w;
                    w = (w / 2).max(1);

                    mipmaps[i].height = h;
                    h = (h / 2).max(1);
                }
            }
        }

        // header_offset / header_length
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
            TextureType::DIRECT => {
                // палитра сразу после HEADER_SIZE
                (HEADER_SIZE as usize, 256 * 4)
            }
        };

        // считаем дырки
        let mut ranges = Vec::new();
        for i in 0..MAX_MIPS {
            let off = mipmaps[i].offset;
            let len = mipmaps[i].length;
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

        Ok(Self {
            version, //
            texture_type,
            compression,
            alpha_bits,
            alpha_type,
            has_mips,
            width,
            height,
            extra,
            has_mipmaps,
            mipmaps: mipmaps.into_iter().collect(),
            holes,
            header_offset,
            header_length,
            source: SourceKind::Blp,
        })
    }
}
