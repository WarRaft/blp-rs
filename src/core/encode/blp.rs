use crate::core::encode::utils::pack_rgba_to_cmyk_fast::pack_rgba_to_cmyk_fast;
use crate::core::encode::utils::pack_rgba_to_rgb_fast::pack_rgba_to_rgb_fast;
use crate::core::encode::utils::read_be_u16::read_be_u16;
use crate::core::encode::utils::rebuild_minimal_jpeg_header::rebuild_minimal_jpeg_header;
use crate::core::image::{ImageBlp, MAX_MIPS};
use crate::error::error::BlpError;
use std::ffi::CStr;
use turbojpeg::{libc, raw};

// === публичные структуры (внешний API) ===
#[derive(Clone)]
pub struct Mip {
    pub w: u32,
    pub h: u32,
    pub visible: bool,
    pub encode_ms: f64,
}

pub struct Ctx {
    pub bytes: Vec<u8>,
    pub mips: Vec<Mip>,
    pub has_alpha: bool,
    pub encode_ms_total: f64,
}

impl ImageBlp {
    pub fn encode_blp(&self, quality: u8, mip_visible: &[bool]) -> Result<Ctx, BlpError> {
        use image::RgbaImage;
        use std::{ptr, time::Instant};

        // --- рабочая структура (заимствуем, без клонов) ---
        struct WorkMip<'a> {
            w: u32,
            h: u32,
            vis: bool,
            img: Option<&'a RgbaImage>, // &RgbaImage, не clone
            encoded: Vec<u8>,           // полный JPEG (sanitized header + scan + EOI)
            encode_ms: f64,
        }

        // 1) находим первый видимый с картинкой
        let total = self.mipmaps.len().min(MAX_MIPS);
        let start_idx = (0..total)
            .find(|&i| {
                mip_visible
                    .get(i)
                    .copied()
                    .unwrap_or(true)
                    && self.mipmaps[i].image.is_some()
            })
            .ok_or_else(|| BlpError::new("no_visible_mips_after_mask"))?;

        // 1.1) собираем work начиная с start_idx (только ссылки)
        let mut work: Vec<WorkMip> = Vec::with_capacity(total - start_idx);
        for i in start_idx..total {
            let m = &self.mipmaps[i];
            work.push(WorkMip {
                w: m.width,
                h: m.height,
                vis: mip_visible
                    .get(i)
                    .copied()
                    .unwrap_or(true),
                img: m.image.as_ref(), // Option<&RgbaImage>
                encoded: Vec::new(),
                encode_ms: 0.0,
            });
        }

        // 2) базовый мип и альфа
        let base_img = work[0]
            .img
            .ok_or_else(|| BlpError::new("first_visible_slot_missing_src"))?;
        if base_img.width() != work[0].w || base_img.height() != work[0].h {
            return Err(BlpError::new("mip.size_mismatch")
                .with_arg("want_w", work[0].w)
                .with_arg("want_h", work[0].h)
                .with_arg("got_w", base_img.width())
                .with_arg("got_h", base_img.height()));
        }
        let has_alpha = base_img.pixels().any(|p| p.0[3] != 255);

        let t0 = Instant::now();

        // 3) кодирование мипов → WorkMip.encoded
        for wm in &mut work {
            if !(wm.vis && wm.img.is_some()) {
                wm.encoded.clear();
                wm.encode_ms = 0.0;
                continue;
            }
            let rgba = wm.img.unwrap();
            let wz = rgba.width() as usize;
            let hz = rgba.height() as usize;

            if wz != wm.w as usize || hz != wm.h as usize {
                return Err(BlpError::new("mip.size_mismatch")
                    .with_arg("want_w", wm.w)
                    .with_arg("want_h", wm.h)
                    .with_arg("got_w", wz)
                    .with_arg("got_h", hz));
            }

            // упаковка под TurboJPEG
            let src = rgba.as_raw();
            let (packed, pitch) = if has_alpha {
                pack_rgba_to_cmyk_fast(src, wz, hz) // pitch = wz * 4
            } else {
                pack_rgba_to_rgb_fast(src, wz, hz) // pitch = wz * 3
            };

            let t_mip = Instant::now();

            // TurboJPEG 3
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
                if raw::tj3Set(
                    handle, //
                    raw::TJPARAM_TJPARAM_COLORSPACE as libc::c_int,
                    if has_alpha { raw::TJCS_TJCS_CMYK } else { raw::TJCS_TJCS_RGB } as libc::c_int,
                ) != 0
                {
                    return Err(tj3_err(handle, "tj3.colorspace"));
                }

                let mut out_ptr: *mut libc::c_uchar = ptr::null_mut();
                let mut out_size: raw::size_t = 0;
                let r = raw::tj3Compress8(
                    handle, //
                    packed.as_ptr(),
                    wz as libc::c_int,
                    pitch as libc::c_int,
                    hz as libc::c_int,
                    if has_alpha { raw::TJPF_TJPF_CMYK } else { raw::TJPF_TJPF_BGR } as libc::c_int,
                    &mut out_ptr,
                    &mut out_size,
                );
                if r != 0 {
                    return Err(tj3_err(handle, "tj3.compress"));
                }
                let slice = std::slice::from_raw_parts(out_ptr, out_size as usize);
                let vec = slice.to_vec();
                raw::tj3Free(out_ptr as *mut libc::c_void);
                vec
            };

            // sanitize header
            let (head_len, _scan_len) = split_header_and_scan(&jpeg_raw)?;
            let header_clean = rebuild_minimal_jpeg_header(&jpeg_raw[..head_len])?;
            wm.encoded = {
                let mut v = Vec::with_capacity(jpeg_raw.len());
                v.extend_from_slice(&header_clean);
                v.extend_from_slice(&jpeg_raw[head_len..]); // scan + EOI
                v
            };
            wm.encode_ms = t_mip.elapsed().as_secs_f64() * 1000.0;
        }

        let encode_ms_total = t0.elapsed().as_secs_f64() * 1000.0;

        // первый видимый обязан быть закодирован
        if work
            .first()
            .map(|m| m.encoded.is_empty())
            .unwrap_or(true)
        {
            return Err(BlpError::new("first_visible_slot_missing"));
        }

        // 4) общий header как общий префикс
        let mut heads: Vec<&[u8]> = Vec::new();
        for m in &work {
            if m.encoded.is_empty() {
                continue;
            }
            let (hlen, _) = split_header_and_scan(&m.encoded)?; // вычисляем на лету
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

        #[inline]
        fn write_u32_le_at(buf: &mut [u8], pos: usize, v: u32) {
            buf[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
        }

        let visible_count = work
            .iter()
            .filter(|m| !m.encoded.is_empty())
            .count();

        // заголовок BLP1
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BLP1");
        bytes.extend_from_slice(&0u32.to_le_bytes()); // compression = 0 (JPEG)
        bytes.extend_from_slice(&(if has_alpha { 8u32 } else { 0u32 }).to_le_bytes()); // flags
        bytes.extend_from_slice(&work[0].w.to_le_bytes());
        bytes.extend_from_slice(&work[0].h.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes()); // extra field
        bytes.extend_from_slice(&(if visible_count > 1 { 1u32 } else { 0u32 }).to_le_bytes()); // has_mipmaps

        // плейсхолдеры offsets / sizes
        let pos_offsets = bytes.len();
        bytes.resize(bytes.len() + MAX_MIPS * 4, 0);
        let pos_sizes = bytes.len();
        bytes.resize(bytes.len() + MAX_MIPS * 4, 0);

        // общий JPEG header
        let jpeg_header_size: u32 = common_header
            .len()
            .try_into()
            .map_err(|_| BlpError::new("jpeg_header_too_large"))?;
        bytes.extend_from_slice(&jpeg_header_size.to_le_bytes());
        bytes.extend_from_slice(&common_header);
        bytes.extend_from_slice(b"RAFT"); // твой маркер

        // payload’ы: строгие проверки вместо debug_assert!
        for i in 0..MAX_MIPS.min(work.len()) {
            let m = &work[i];
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

        // 5) внешний список мипов (без байтов)
        let mut out_mips: Vec<Mip> = Vec::with_capacity(work.len());
        for wm in &work {
            out_mips.push(Mip { w: wm.w, h: wm.h, visible: wm.vis, encode_ms: wm.encode_ms });
        }

        Ok(Ctx { bytes, mips: out_mips, has_alpha, encode_ms_total })
    }
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
            0xD0..=0xD7 | 0x01 => {} // no length
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

fn header_prefix(heads: &[&[u8]]) -> Vec<u8> {
    if heads.is_empty() {
        return Vec::new();
    }
    let min_len = heads
        .iter()
        .map(|h| h.len())
        .min()
        .unwrap();
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
