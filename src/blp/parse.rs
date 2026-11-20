use crate::{
    BlpError,
    blp::{Blp, Frame, MAX_MIPS, TextureType, Version},
};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

/// Size of the palette: 256 BGRA entries.
const PALETTE_SIZE: usize = 256 * 4;

/// See [`Blp::parse`].
///
/// ```text
/// BLP0/BLP1: magic, u32 type, u32 alpha bits, u32 width, u32 height,
///            u32 extra, u32 has mipmaps            (28 bytes)
/// BLP2:      magic, u32 type, u8 compression, u8 alpha bits,
///            u8 alpha type, u8 has mipmaps, u32 width, u32 height (20 bytes)
/// then, BLP1/BLP2: u32 mip offsets[16], u32 mip lengths[16]
/// then a palette (1024 bytes), or u32 JPEG header length + the header.
/// ```
///
/// BLP0 keeps each mip in a file of its own, so its mips have no data here.
pub(crate) fn parse(buf: &[u8]) -> Result<(Blp, Vec<Frame>), BlpError> {
    let mut c = Cursor::new(buf);
    let truncated = |e: std::io::Error| BlpError::new("blp.header.truncated").push_std(e);

    let version = Version::try_from(
        c.read_u32::<BigEndian>()
            .map_err(truncated)?,
    )?;
    let texture_type = TextureType::try_from(
        c.read_u32::<LittleEndian>()
            .map_err(truncated)?,
    )?;

    let (compression, alpha_bits, alpha_type, mips_flag) = if version == Version::BLP2 {
        let mut b = [0u8; 4];
        std::io::Read::read_exact(&mut c, &mut b).map_err(truncated)?;
        (b[0], b[1] as u32, b[2], b[3] as u32)
    } else {
        (
            0,
            c.read_u32::<LittleEndian>()
                .map_err(truncated)?,
            0,
            0,
        )
    };

    let width = c
        .read_u32::<LittleEndian>()
        .map_err(truncated)?;
    let height = c
        .read_u32::<LittleEndian>()
        .map_err(truncated)?;
    if width == 0 || height == 0 {
        return Err(BlpError::new("blp.size.zero")
            .with_arg("width", width)
            .with_arg("height", height));
    }

    let (extra, has_mipmaps) = if version == Version::BLP2 {
        (0, mips_flag)
    } else {
        (
            c.read_u32::<LittleEndian>()
                .map_err(truncated)?,
            c.read_u32::<LittleEndian>()
                .map_err(truncated)?,
        )
    };

    // Halving down to 1×1, as many as the header has slots for.
    let levels = ((u32::BITS - width.max(height).leading_zeros()) as usize).min(MAX_MIPS);
    let mut frames: Vec<Frame> = (0..levels)
        .map(|i| Frame { width: (width >> i).max(1), height: (height >> i).max(1), offset: 0, length: 0 })
        .collect();

    if version != Version::BLP0 {
        let mut offsets = [0u32; MAX_MIPS];
        let mut lengths = [0u32; MAX_MIPS];
        c.read_u32_into::<LittleEndian>(&mut offsets)
            .map_err(truncated)?;
        c.read_u32_into::<LittleEndian>(&mut lengths)
            .map_err(truncated)?;
        for (i, frame) in frames.iter_mut().enumerate() {
            frame.offset = offsets[i] as usize;
            frame.length = lengths[i] as usize;
        }
        while frames.len() > 1
            && frames
                .last()
                .is_some_and(|f| f.length == 0)
        {
            frames.pop();
        }
    }

    let base = c.position() as usize;
    let header = match texture_type {
        TextureType::JPEG => {
            let len = c
                .read_u32::<LittleEndian>()
                .map_err(truncated)? as usize;
            Frame { width: 0, height: 0, offset: base + 4, length: len }
        }
        TextureType::PALETTE => Frame { width: 0, height: 0, offset: base, length: PALETTE_SIZE },
    };
    // DXT and BGRA textures (BLP2) still carry a palette, unused.
    header
        .data(buf)
        .map_err(|e| e.ctx("blp.header.oob"))?;

    let holes = holes(buf.len(), header.offset + header.length, &frames);
    let blp = Blp { version, texture_type, compression, alpha_bits, alpha_type, width, height, extra, has_mipmaps, holes, header };
    Ok((blp, frames))
}

/// Bytes after the header that no mip covers.
fn holes(file: usize, header_end: usize, frames: &[Frame]) -> usize {
    let mut ranges: Vec<(usize, usize)> = frames
        .iter()
        .filter(|f| f.length > 0)
        .filter_map(|f| {
            f.offset
                .checked_add(f.length)
                .filter(|&end| end <= file)
                .map(|end| (f.offset, end))
        })
        .collect();
    ranges.sort_unstable();

    let mut covered_to = header_end;
    let mut holes = 0;
    for (start, end) in ranges {
        holes += start.saturating_sub(covered_to);
        covered_to = covered_to.max(end);
    }
    holes + file.saturating_sub(covered_to)
}
