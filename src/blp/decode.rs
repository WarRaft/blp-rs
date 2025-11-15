use crate::blp::Blp;
use crate::error::error::BlpError;
use image::RgbaImage;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use jpeg_decoder::{Decoder, PixelFormat};

/// Delegate to decode JPEG-based mips as Blp helper.
pub fn decode_jpeg_to_mipmaps(img: &Blp, buf: &[u8], mip_visible: &[bool]) -> Result<Vec<Option<RgbaImage>>, BlpError> {
    // This implementation is copied from src/_decode/jpeg.rs; keep logically the same.
    // --- Validate header range and slice it out ---
    let h_off = img.header.offset;
    let h_len = img.header.length;
    if h_off.checked_add(h_len).is_none() || h_off + h_len > buf.len() {
        return Err(BlpError::new("jpeg.header.oob"));
    }
    let header_bytes = &buf[h_off..h_off + h_len];

    // If alpha_bits == 0 we force opaque alpha channel when reconstructing RGBA.
    let force_opaque = img.alpha_bits == 0;

    // --- Walk over mip chain ---
    let mut out: Vec<Option<RgbaImage>> = Vec::with_capacity(img.frames.len());
    for i in 0..img.frames.len() {
        let visible = mip_visible.get(i).copied().unwrap_or(true);
        if !visible { out.push(None); continue; }
        let off = img.frames[i].offset;
        let len = img.frames[i].length;
        if len == 0 { out.push(None); continue; }
        if off.checked_add(len).is_none() || off + len > buf.len() { out.push(None); continue; }

        let tail = &buf[off..off + len];
        let mut full = Vec::with_capacity(header_bytes.len() + tail.len());
        full.extend_from_slice(header_bytes);
        full.extend_from_slice(tail);

        let mut dec = Decoder::new(Cursor::new(&full));
        dec.read_info().map_err(|e| BlpError::from(e).with_arg("phase", "read_info").with_arg("mip", i as u32))?;
        let info = dec.info().ok_or_else(|| BlpError::new("jpeg.meta.missing").with_arg("mip", i as u32))?;
        let (w, h) = (info.width as u32, info.height as u32);
        let pixels = dec.decode().map_err(|e| BlpError::from(e).with_arg("phase", "decode").with_arg("mip", i as u32))?;

        let mut imgbuf = RgbaImage::new(w, h);
        match info.pixel_format {
            PixelFormat::CMYK32 => {
                for (p, px) in imgbuf.pixels_mut().enumerate() {
                    let idx = p * 4; let c = pixels[idx]; let m = pixels[idx + 1]; let y = pixels[idx + 2]; let k = pixels[idx + 3];
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
                    let l = pixels[p]; *px = image::Rgba([l, l, l, 255]);
                }
            }
            PixelFormat::L16 => {
                for (chunk, px) in pixels.chunks_exact(2).zip(imgbuf.pixels_mut()) {
                    let l16 = u16::from_be_bytes([chunk[0], chunk[1]]);
                    let l8 = (l16 / 257) as u8; *px = image::Rgba([l8, l8, l8, 255]);
                }
            }
        }

        if img.frames[i].width == w && img.frames[i].height == h { out.push(Some(imgbuf)); }
        else if let Some(level) = (0..img.frames.len()).find(|&lvl| img.frames[lvl].width == w && img.frames[lvl].height == h) {
            while out.len() <= level { out.push(None); }
            out[level] = Some(imgbuf);
        } else { out.push(Some(imgbuf)); }
    }
    while out.len() < img.frames.len() { out.push(None); }
    Ok(out)
}

/// Decode DIRECT (paletted) BLP into images.
pub fn decode_direct_to_mipmaps(img: &Blp, buf: &[u8], mip_visible: &[bool]) -> Result<Vec<Option<RgbaImage>>, BlpError> {
    use std::io::Read;
    if img.header.offset + img.header.length > buf.len() { return Err(BlpError::new("direct.header.oob")); }
    let mut cur = Cursor::new(&buf[..]); cur.set_position(img.header.offset as u64);
    let mut palette = [[0u8; 3]; 256];
    for i in 0..256 { let color = cur.read_u32::<LittleEndian>()?; let r = ((color >> 16) & 0xFF) as u8; let g = ((color >> 8) & 0xFF) as u8; let b = (color & 0xFF) as u8; palette[i] = [r, g, b]; }

    let mut out: Vec<Option<RgbaImage>> = Vec::with_capacity(img.frames.len());
    let buf_len = buf.len(); let alpha_bits = img.alpha_bits;
    for i in 0..img.frames.len() {
        let visible = mip_visible.get(i).copied().unwrap_or(true);
        if !visible { out.push(None); continue; }
        let off = img.frames[i].offset; let len = img.frames[i].length;
        if len == 0 { out.push(None); continue; }
        if off.checked_add(len).is_none() || off + len > buf_len { continue; }
        cur.set_position(off as u64);
        let (w, h) = (img.frames[i].width, img.frames[i].height);
        let pixel_count = (w as usize) * (h as usize);
        let mut indices = vec![0u8; pixel_count]; cur.read_exact(&mut indices).map_err(|_| BlpError::new("direct.indices.truncated"))?;
        let alpha_bytes = match alpha_bits { 0 => 0, 1 => (pixel_count + 7) / 8, 4 => (pixel_count + 1) / 2, 8 => pixel_count, _ => return Err(BlpError::new("blp.version.invalid").with_arg("msg", "unsupported alpha bits")), };
        let mut alpha_raw = vec![0u8; alpha_bytes]; if alpha_bytes > 0 { cur.read_exact(&mut alpha_raw).map_err(|_| BlpError::new("direct.alpha.truncated"))?; }
        let mut out_img = RgbaImage::new(w, h);
        for p in 0..pixel_count { let idx = indices[p] as usize; let [r, g, b] = palette[idx]; let a = match alpha_bits { 0 => 255, 1 => { let byte = alpha_raw[p / 8]; let bit = (byte >> (p % 8)) & 1; if bit == 1 { 255 } else { 0 } }, 4 => { let byte = alpha_raw[p / 2]; let nibble = if (p & 1) == 0 { byte & 0x0F } else { byte >> 4 }; (nibble << 4) | nibble }, 8 => alpha_raw[p], _ => 255, }; out_img.get_pixel_mut((p as u32) % w, (p as u32) / w).0 = [r, g, b, a]; }
        out.push(Some(out_img));
    }
    while out.len() < img.frames.len() { out.push(None); }
    Ok(out)
}
