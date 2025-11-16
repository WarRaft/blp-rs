use crate::blp::{Blp, Frame, MAX_MIPS};
use crate::error::error::BlpError;
use std::ffi::CStr;
use turbojpeg::{libc, raw};

// ============================================================================
// Encoding utility functions
// ============================================================================

/// Convert RGBA to CMYK format for JPEG encoding.
/// If has_alpha is false, fills the K (alpha) channel with 255.
#[inline(always)]
fn pack_rgba_to_cmyk_fast(src: &[u8], w: usize, h: usize, has_alpha: bool) -> (Vec<u8>, usize) {
    debug_assert_eq!(src.len(), w * h * 4);

    let mut out = vec![0u8; w * h * 4];
    let mut si = 0usize; // step 4
    let mut di = 0usize; // step 4

    // RGBA -> CMYK  (C=B, M=G, Y=R, K=A)
    while si < src.len() {
        out[di] = src[si + 2]; // C ← B
        out[di + 1] = src[si + 1]; // M ← G
        out[di + 2] = src[si]; // Y ← R
        out[di + 3] = if has_alpha { src[si + 3] } else { 255 }; // K ← A or 255
        si += 4;
        di += 4;
    }
    (out, w * 4)
}

/// Convert two bytes into u16 big-endian.
#[inline]
fn read_be_u16(b: &[u8]) -> Result<u16, BlpError> {
    if b.len() < 2 {
        return Err(BlpError::new("jpeg.len"));
    }
    Ok(((b[0] as u16) << 8) | b[1] as u16)
}

/// Rebuild a minimal JPEG header that keeps SOI, non-APP/COM markers and the first SOF + SOS.
#[inline]
fn rebuild_minimal_jpeg_header(header: &[u8]) -> Result<Vec<u8>, BlpError> {
    // Validate SOI
    if header.len() < 4 || header[0] != 0xFF || header[1] != 0xD8 {
        return Err(BlpError::new("jpeg.bad_soi"));
    }

    let mut pos = 2usize;
    let mut others: Vec<(usize, usize)> = Vec::new();
    let mut sof_seg: Option<(usize, usize)> = None;
    let mut sos_seg: Option<(usize, usize)> = None;

    while pos < header.len() {
        while pos < header.len() && header[pos] == 0xFF {
            pos += 1;
        }
        if pos >= header.len() {
            break;
        }

        let id = header[pos];
        let start = pos - 1; // points at 0xFF
        pos += 1;

        if id == 0x01 || (0xD0..=0xD7).contains(&id) {
            others.push((start, pos));
            continue;
        }

        if pos + 2 > header.len() {
            return Err(BlpError::new("jpeg.seg_len"));
        }
        let seg_len = read_be_u16(&header[pos..pos + 2])? as usize;
        let end = pos + seg_len;
        if end > header.len() {
            return Err(BlpError::new("jpeg.seg_trunc"));
        }

        if id == 0xDA {
            sos_seg = Some((start, end));
            break;
        } else if (0xE0..=0xEF).contains(&id) || id == 0xFE {
            // APPn and COM – skip
        } else if (0xC0..=0xCF).contains(&id) && id != 0xC4 && id != 0xC8 {
            if sof_seg.is_none() {
                sof_seg = Some((start, end));
            }
        } else {
            others.push((start, end));
        }

        pos = end;
    }

    let (sos_s, sos_e) = sos_seg.ok_or_else(|| BlpError::new("jpeg.sos_missing"))?;
    let (sof_s, sof_e) = sof_seg.ok_or_else(|| BlpError::new("jpeg.sof_missing"))?;

    let mut out = Vec::with_capacity(header.len());
    out.extend_from_slice(&header[..2]); // SOI
    for (s, e) in others {
        out.extend_from_slice(&header[s..e]);
    }
    out.extend_from_slice(&header[sof_s..sof_e]);
    out.extend_from_slice(&header[sos_s..sos_e]);
    Ok(out)
}

// ============================================================================
// BLP encoding implementation
// ============================================================================

/// Create a Blp header from raw RGBA data.
///
/// This is typically used as the first step in BLP encoding.
impl Blp {
    /// Encode RGBA pixel data to BLP format.
    ///
    /// This function:
    /// 1. Takes raw RGBA pixels and dimensions
    /// 2. Scales to power-of-two dimensions if needed
    /// 3. Generates mipmaps according to mip_visible mask
    /// 4. Encodes to BLP with specified quality
    ///
    /// # Arguments
    /// * `rgba` - Raw RGBA pixel data (width * height * 4 bytes)
    /// * `width` - Image width
    /// * `height` - Image height  
    /// * `quality` - JPEG quality (1-100)
    /// * `mip_visible` - Mask of which mipmaps to generate (length 16)
    pub fn encode_from_rgba(rgba: &[u8], width: u32, height: u32, quality: u8, mip_visible: &[bool]) -> Result<Vec<u8>, BlpError> {
        use image::RgbaImage;

        /// Round up to the nearest power of two.
        fn next_pow2(v: u32) -> u32 {
            if v == 0 {
                return 1;
            }
            let mut n = v - 1;
            n |= n >> 1;
            n |= n >> 2;
            n |= n >> 4;
            n |= n >> 8;
            n |= n >> 16;
            n + 1
        }

        // Validate input
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

        // Create base image
        let base_rgba = RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or_else(|| BlpError::new("error-rgba-image-creation"))?;

        // Calculate target power-of-two dimensions
        let target_w = next_pow2(width);
        let target_h = next_pow2(height);

        // Resize to power-of-two if needed
        let base_img = if width != target_w || height != target_h {
            image::imageops::resize(&base_rgba, target_w, target_h, image::imageops::FilterType::Lanczos3)
        } else {
            base_rgba
        };

        // Build power-of-two mip chain
        let levels = (32 - target_w.max(target_h).leading_zeros()) as usize;
        let mut frames: Vec<Frame> = Vec::with_capacity(MAX_MIPS);
        let (mut w, mut h) = (target_w, target_h);
        for i in 0..MAX_MIPS {
            if i < levels {
                frames.push(Frame { width: w, height: h, offset: 0, length: 0 });
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            } else {
                frames.push(Frame::default());
            }
        }

        // Generate mipmaps
        let mut frame_images: Vec<Option<RgbaImage>> = vec![None; frames.len()];
        frame_images[0] = Some(base_img.clone());

        let mut prev = base_img;
        let mut w = target_w;
        let mut h = target_h;

        for i in 1..frames.len() {
            if !mip_visible
                .get(i)
                .copied()
                .unwrap_or(true)
            {
                break;
            }

            let next_w = (w / 2).max(1);
            let next_h = (h / 2).max(1);

            let next_img = image::imageops::resize(&prev, next_w, next_h, image::imageops::FilterType::Lanczos3);

            frame_images[i] = Some(next_img.clone());
            prev = next_img;
            w = next_w;
            h = next_h;

            if w == 1 && h == 1 {
                break;
            }
        }

        // Encode mipmaps to JPEG
        use std::ptr;

        struct WorkMip {
            w: u32,
            h: u32,
            encoded: Vec<u8>,
        }

        let total = frames.len().min(MAX_MIPS);
        let start_idx = (0..total)
            .find(|&i| {
                mip_visible
                    .get(i)
                    .copied()
                    .unwrap_or(true)
                    && frame_images
                        .get(i)
                        .cloned()
                        .unwrap_or(None)
                        .is_some()
            })
            .ok_or_else(|| BlpError::new("no_visible_mips_after_mask"))?;

        let mut work: Vec<WorkMip> = Vec::with_capacity(total - start_idx);
        for i in start_idx..total {
            let m = &frames[i];
            if !mip_visible
                .get(i)
                .copied()
                .unwrap_or(true)
                || frame_images
                    .get(i)
                    .cloned()
                    .unwrap_or(None)
                    .is_none()
            {
                continue;
            }
            work.push(WorkMip { w: m.width, h: m.height, encoded: Vec::new() });
        }

        let base_img_ref = frame_images[start_idx]
            .as_ref()
            .unwrap();
        let has_alpha = base_img_ref
            .pixels()
            .any(|p| p.0[3] != 255);

        for (idx, wm) in work.iter_mut().enumerate() {
            let actual_idx = start_idx + idx;
            let rgba = frame_images[actual_idx]
                .as_ref()
                .unwrap();
            let wz = rgba.width() as usize;
            let hz = rgba.height() as usize;

            if wz != wm.w as usize || hz != wm.h as usize {
                return Err(BlpError::new("mip.size_mismatch")
                    .with_arg("want_w", wm.w)
                    .with_arg("want_h", wm.h)
                    .with_arg("got_w", wz)
                    .with_arg("got_h", hz));
            }

            let src = rgba.as_raw();
            let (packed, pitch) = pack_rgba_to_cmyk_fast(src, wz, hz, has_alpha);

            let handle = unsafe { raw::tj3Init(raw::TJINIT_TJINIT_COMPRESS as libc::c_int) };
            if handle.is_null() {
                return Err(BlpError::new("tj3.init"));
            }
            let jpeg_raw = unsafe {
                struct Guard(raw::tjhandle);
                impl Drop for Guard {
                    fn drop(&mut self) {
                        if !self.0.is_null() {
                            unsafe { raw::tj3Destroy(self.0) };
                        }
                    }
                }
                let _g = Guard(handle);

                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_QUALITY as libc::c_int, quality as libc::c_int) != 0 {
                    return Err(tj3_err(handle, "tj3.quality"));
                }
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_SUBSAMP as libc::c_int, raw::TJSAMP_TJSAMP_444 as libc::c_int) != 0 {
                    return Err(tj3_err(handle, "tj3.subsamp"));
                }
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_OPTIMIZE as libc::c_int, 0) != 0 {
                    return Err(tj3_err(handle, "tj3.optimize"));
                }
                if raw::tj3Set(handle, raw::TJPARAM_TJPARAM_COLORSPACE as libc::c_int, raw::TJCS_TJCS_CMYK as libc::c_int) != 0 {
                    return Err(tj3_err(handle, "tj3.colorspace"));
                }

                let mut out_ptr: *mut libc::c_uchar = ptr::null_mut();
                let mut out_size: raw::size_t = 0;
                let r = raw::tj3Compress8(handle, packed.as_ptr(), wz as libc::c_int, pitch as libc::c_int, hz as libc::c_int, raw::TJPF_TJPF_CMYK as libc::c_int, &mut out_ptr, &mut out_size);
                if r != 0 {
                    return Err(tj3_err(handle, "tj3.compress"));
                }
                let slice = std::slice::from_raw_parts(out_ptr, out_size as usize);
                let vec = slice.to_vec();
                raw::tj3Free(out_ptr as *mut libc::c_void);
                vec
            };

            let (head_len, _scan_len) = split_header_and_scan(&jpeg_raw)?;
            let header_clean = rebuild_minimal_jpeg_header(&jpeg_raw[..head_len])?;
            wm.encoded = {
                let mut v = Vec::with_capacity(jpeg_raw.len());
                v.extend_from_slice(&header_clean);
                v.extend_from_slice(&jpeg_raw[head_len..]);
                v
            };
        }

        if work
            .first()
            .map(|m| m.encoded.is_empty())
            .unwrap_or(true)
        {
            return Err(BlpError::new("first_visible_slot_missing"));
        }

        // Find common JPEG header
        let mut heads: Vec<&[u8]> = Vec::new();
        for m in &work {
            if m.encoded.is_empty() {
                continue;
            }
            let (hlen, _) = split_header_and_scan(&m.encoded)?;
            heads.push(&m.encoded[..hlen]);
        }
        if heads.is_empty() {
            return Err(BlpError::new("no_encoded_heads"));
        }

        let mut common_header = header_prefix(&heads);
        if common_header.len() < 2 || common_header[0] != 0xFF || common_header[1] != 0xD8 {
            return Err(BlpError::new("bad_common_header"));
        }
        for h in &heads {
            while !h.starts_with(&common_header) && !common_header.is_empty() {
                common_header.pop();
            }
            if !h.starts_with(&common_header) {
                return Err(BlpError::new("head_prefix_mismatch"));
            }
        }

        // Build BLP file
        #[inline]
        fn write_u32_le_at(buf: &mut [u8], pos: usize, v: u32) {
            buf[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
        }

        let visible_count = work
            .iter()
            .filter(|m| !m.encoded.is_empty())
            .count();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BLP1");
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(if has_alpha { 8u32 } else { 0u32 }).to_le_bytes());
        bytes.extend_from_slice(&target_w.to_le_bytes());
        bytes.extend_from_slice(&target_h.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(if visible_count > 1 { 1u32 } else { 0u32 }).to_le_bytes());

        let pos_offsets = bytes.len();
        bytes.resize(bytes.len() + MAX_MIPS * 4, 0);
        let pos_sizes = bytes.len();
        bytes.resize(bytes.len() + MAX_MIPS * 4, 0);

        let jpeg_header_size: u32 = common_header
            .len()
            .try_into()
            .map_err(|_| BlpError::new("jpeg_header_too_large"))?;
        bytes.extend_from_slice(&jpeg_header_size.to_le_bytes());
        bytes.extend_from_slice(&common_header);
        bytes.extend_from_slice(b"RAFT");

        for (idx, m) in work.iter().enumerate() {
            let i = start_idx + idx;
            if m.encoded.is_empty() {
                continue;
            }

            let (head_len, _) = split_header_and_scan(&m.encoded)?;
            if head_len < common_header.len() {
                return Err(BlpError::new("mip.head_too_short")
                    .with_arg("mip", i)
                    .with_arg("head_len", head_len)
                    .with_arg("common_len", common_header.len()));
            }
            if &m.encoded[..common_header.len()] != &common_header[..] {
                return Err(BlpError::new("mip.common_header_mismatch")
                    .with_arg("mip", i)
                    .with_arg("head_prefix", hex::encode(&m.encoded[..common_header.len()]))
                    .with_arg("common_prefix", hex::encode(&common_header)));
            }

            let payload = &m.encoded[common_header.len()..];
            let off = bytes.len();
            let sz = payload.len();

            if off > u32::MAX as usize {
                return Err(BlpError::new("offset_too_large"));
            }
            if sz > u32::MAX as usize {
                return Err(BlpError::new("payload_too_large"));
            }

            write_u32_le_at(&mut bytes, pos_offsets + (i << 2), off as u32);
            write_u32_le_at(&mut bytes, pos_sizes + (i << 2), sz as u32);

            bytes.extend_from_slice(payload);
        }

        Ok(bytes)
    }
}

fn header_prefix(heads: &[&[u8]]) -> Vec<u8> {
    if heads.is_empty() {
        return Vec::new();
    }
    let min_len = heads
        .iter()
        .map(|h| h.len())
        .min()
        .unwrap_or(0);
    let mut out = Vec::with_capacity(min_len);
    for i in 0..min_len {
        let b = heads[0][i];
        if heads.iter().all(|h| h[i] == b) {
            out.push(b);
        } else {
            break;
        }
    }
    out
}

fn tj3_err(handle: raw::tjhandle, key: &'static str) -> BlpError {
    let msg = unsafe {
        let p = raw::tj3GetErrorStr(handle);
        if p.is_null() {
            "unknown".to_string()
        } else {
            CStr::from_ptr(p)
                .to_string_lossy()
                .into_owned()
        }
    };
    BlpError::new(key).with_arg("msg", msg)
}

fn split_header_and_scan(jpeg: &[u8]) -> Result<(usize, usize), BlpError> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return Err(BlpError::new("jpeg.bad_soi"));
    }
    let mut i = 2usize;
    loop {
        while i < jpeg.len() && jpeg[i] == 0xFF {
            i += 1;
        }
        if i >= jpeg.len() {
            return Err(BlpError::new("jpeg.truncated"));
        }
        let m = jpeg[i];
        i += 1;
        match m {
            0xD9 => return Err(BlpError::new("jpeg.eoi_before_sos")),
            0xD0..=0xD7 | 0x01 => {}
            0xDA => {
                if i + 2 > jpeg.len() {
                    return Err(BlpError::new("jpeg.sos_len"));
                }
                let seg_len = read_be_u16(&jpeg[i..i + 2])? as usize;
                let seg_end = i + seg_len;
                if seg_end > jpeg.len() {
                    return Err(BlpError::new("jpeg.sos_trunc"));
                }
                let head_len = seg_end;
                let mut j = head_len;
                while j + 1 < jpeg.len() {
                    if jpeg[j] == 0xFF {
                        let n = jpeg[j + 1];
                        if n == 0x00 || (0xD0..=0xD7).contains(&n) {
                            j += 2;
                            continue;
                        }
                        if n == 0xD9 {
                            return Ok((head_len, j - head_len));
                        }
                    }
                    j += 1;
                }
                return Err(BlpError::new("jpeg.eoi_not_found"));
            }
            _ => {
                if i + 2 > jpeg.len() {
                    return Err(BlpError::new("jpeg.seg_len"));
                }
                let seg_len = read_be_u16(&jpeg[i..i + 2])? as usize;
                let seg_end = i + seg_len;
                if seg_end > jpeg.len() {
                    return Err(BlpError::new("jpeg.seg_trunc"));
                }
                i = seg_end;
            }
        }
    }
}
