use crate::{
    BlpError,
    blp::{Blp, MAX_MIPS},
};
use image::{RgbaImage, imageops::FilterType};
use std::ffi::CStr;
use turbojpeg::{libc, raw};

/// Written between the shared JPEG header and the first mip.
const SIGNATURE: &[u8] = b"RAFT";

impl Blp {
    /// Encodes RGBA pixels as a BLP1 JPEG texture.
    ///
    /// A side that isn't a power of two is scaled up to the next one. Mip
    /// `i` is produced when `mip_visible[i]` is true or missing, each one
    /// halving the one before, down to 1×1. `quality` is JPEG's, 1–100.
    pub fn encode_from_rgba(rgba: &[u8], width: u32, height: u32, quality: u8, mip_visible: &[bool]) -> Result<Vec<u8>, BlpError> {
        if width == 0 || height == 0 {
            return Err(BlpError::new("encode.image-empty")
                .with_arg("width", width)
                .with_arg("height", height));
        }
        if quality > 100 {
            return Err(BlpError::new("encode.quality").with_arg("quality", quality));
        }
        let base = RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or_else(|| {
            BlpError::new("encode.rgba-size")
                .with_arg("expected", width as usize * height as usize * 4)
                .with_arg("actual", rgba.len())
        })?;

        let (tw, th) = (width.next_power_of_two(), height.next_power_of_two());
        let base = if (tw, th) != (width, height) {
            image::imageops::resize(&base, tw, th, FilterType::Lanczos3)
        } else {
            base
        };

        let levels = ((u32::BITS - tw.max(th).leading_zeros()) as usize).min(MAX_MIPS);
        let visible = |i: usize| {
            mip_visible
                .get(i)
                .copied()
                .unwrap_or(true)
        };
        let last = (0..levels)
            .rev()
            .find(|&i| visible(i))
            .ok_or_else(|| BlpError::new("encode.no-mips"))?;
        let has_alpha = base.pixels().any(|p| p[3] != 255);

        // Every level down to the last wanted one, each from the one above.
        let mut mips: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut level = base;
        for i in 0..=last {
            if i > 0 {
                level = image::imageops::resize(&level, (tw >> i).max(1), (th >> i).max(1), FilterType::Lanczos3);
            }
            if visible(i) {
                let jpeg = compress_cmyk(&level, quality.max(1), has_alpha)?;
                let head_len = header_len(&jpeg)?;
                let mut mip = minimal_header(&jpeg[..head_len])?;
                mip.extend_from_slice(&jpeg[head_len..]);
                mips.push((i, mip));
            }
        }

        // The header every mip shares, stored once.
        let heads = mips
            .iter()
            .map(|(_, m)| header_len(m).map(|n| &m[..n]))
            .collect::<Result<Vec<_>, _>>()?;
        let shared = common_prefix(&heads);
        if !shared.starts_with(&[0xFF, 0xD8]) {
            return Err(BlpError::new("encode.shared-header"));
        }

        let mut out = Vec::new();
        out.extend_from_slice(b"BLP1");
        for v in [0, if has_alpha { 8 } else { 0 }, tw, th, 0, (mips.len() > 1) as u32] {
            out.extend_from_slice(&u32::to_le_bytes(v));
        }
        let table = out.len();
        out.resize(table + MAX_MIPS * 8, 0);
        out.extend_from_slice(&(shared.len() as u32).to_le_bytes());
        out.extend_from_slice(shared);
        out.extend_from_slice(SIGNATURE);

        for (i, mip) in &mips {
            let payload = &mip[shared.len()..];
            let offset = u32::try_from(out.len()).map_err(|_| BlpError::new("encode.too-large"))?;
            out[table + i * 4..][..4].copy_from_slice(&offset.to_le_bytes());
            out[table + (MAX_MIPS + i) * 4..][..4].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(payload);
        }
        Ok(out)
    }
}

/// JPEG-compresses an image as CMYK holding B, G, R, A (A = 255 without
/// alpha), 4:4:4, the way the game reads it.
fn compress_cmyk(img: &RgbaImage, quality: u8, has_alpha: bool) -> Result<Vec<u8>, BlpError> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let cmyk: Vec<u8> = img
        .pixels()
        .flat_map(|p| [p[2], p[1], p[0], if has_alpha { p[3] } else { 255 }])
        .collect();

    struct Handle(raw::tjhandle);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { raw::tj3Destroy(self.0) };
        }
    }

    // SAFETY: the handle is checked for null and destroyed by the guard;
    // `cmyk` outlives the call and holds `w * h * 4` bytes with pitch `w * 4`;
    // the output buffer is copied, then freed with tj3Free.
    unsafe {
        let handle = raw::tj3Init(raw::TJINIT_TJINIT_COMPRESS as libc::c_int);
        if handle.is_null() {
            return Err(BlpError::new("tj3.init"));
        }
        let handle = Handle(handle);
        let h_ = handle.0;
        for (param, value, key) in [(raw::TJPARAM_TJPARAM_QUALITY, quality as libc::c_int, "tj3.quality"), (raw::TJPARAM_TJPARAM_SUBSAMP, raw::TJSAMP_TJSAMP_444 as libc::c_int, "tj3.subsamp"), (raw::TJPARAM_TJPARAM_OPTIMIZE, 0, "tj3.optimize"), (raw::TJPARAM_TJPARAM_COLORSPACE, raw::TJCS_TJCS_CMYK as libc::c_int, "tj3.colorspace")] {
            if raw::tj3Set(h_, param as libc::c_int, value) != 0 {
                return Err(tj3_err(h_, key));
            }
        }
        let mut out_ptr: *mut libc::c_uchar = std::ptr::null_mut();
        let mut out_size: raw::size_t = 0;
        if raw::tj3Compress8(h_, cmyk.as_ptr(), w as libc::c_int, (w * 4) as libc::c_int, h as libc::c_int, raw::TJPF_TJPF_CMYK as libc::c_int, &mut out_ptr, &mut out_size) != 0 {
            return Err(tj3_err(h_, "tj3.compress"));
        }
        let jpeg = std::slice::from_raw_parts(out_ptr, out_size as usize).to_vec();
        raw::tj3Free(out_ptr as *mut libc::c_void);
        Ok(jpeg)
    }
}

unsafe fn tj3_err(handle: raw::tjhandle, key: &'static str) -> BlpError {
    let p = unsafe { raw::tj3GetErrorStr(handle) };
    let msg = if p.is_null() {
        "unknown".into()
    } else {
        unsafe { CStr::from_ptr(p) }
            .to_string_lossy()
            .into_owned()
    };
    BlpError::new(key).with_arg("msg", msg)
}

/// The longest prefix all the slices share.
fn common_prefix<'a>(heads: &[&'a [u8]]) -> &'a [u8] {
    let Some((first, rest)) = heads.split_first() else { return &[] };
    let len = rest.iter().fold(first.len(), |len, h| {
        first[..len]
            .iter()
            .zip(h.iter())
            .take_while(|(a, b)| a == b)
            .count()
    });
    &first[..len]
}

fn be_u16(b: &[u8], at: usize) -> Result<usize, BlpError> {
    b.get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]) as usize)
        .ok_or_else(|| BlpError::new("jpeg.truncated"))
}

/// The length of a JPEG's header: everything up to and including the SOS
/// segment, where the entropy-coded scan begins.
fn header_len(jpeg: &[u8]) -> Result<usize, BlpError> {
    if !jpeg.starts_with(&[0xFF, 0xD8]) {
        return Err(BlpError::new("jpeg.bad-soi"));
    }
    let mut i = 2;
    loop {
        while jpeg.get(i) == Some(&0xFF) {
            i += 1;
        }
        let marker = *jpeg
            .get(i)
            .ok_or_else(|| BlpError::new("jpeg.truncated"))?;
        i += 1;
        match marker {
            0xD9 => return Err(BlpError::new("jpeg.eoi-before-sos")),
            0x01 | 0xD0..=0xD7 => {}
            _ => {
                let end = i + be_u16(jpeg, i)?;
                if end > jpeg.len() {
                    return Err(BlpError::new("jpeg.segment-truncated"));
                }
                if marker == 0xDA {
                    return Ok(end);
                }
                i = end;
            }
        }
    }
}

/// A header with only what decoding needs: SOI, the tables, the first SOF
/// and SOS; APPn and COM segments dropped, so mips' headers match.
fn minimal_header(header: &[u8]) -> Result<Vec<u8>, BlpError> {
    let mut tables: Vec<&[u8]> = Vec::new();
    let mut sof = None;
    let mut i = 2;
    while i < header.len() {
        while header.get(i) == Some(&0xFF) {
            i += 1;
        }
        let Some(&marker) = header.get(i) else { break };
        let start = i - 1;
        i += 1;
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            tables.push(&header[start..i]);
            continue;
        }
        let end = i + be_u16(header, i)?;
        let segment = header
            .get(start..end)
            .ok_or_else(|| BlpError::new("jpeg.segment-truncated"))?;
        match marker {
            0xDA => {
                let sof = sof.ok_or_else(|| BlpError::new("jpeg.sof-missing"))?;
                let mut out = header[..2].to_vec();
                tables
                    .iter()
                    .for_each(|t| out.extend_from_slice(t));
                out.extend_from_slice(sof);
                out.extend_from_slice(segment);
                return Ok(out);
            }
            0xE0..=0xEF | 0xFE => {}
            0xC0..=0xCF if marker != 0xC4 && marker != 0xC8 => {
                sof.get_or_insert(segment);
            }
            _ => tables.push(segment),
        }
        i = end;
    }
    Err(BlpError::new("jpeg.sos-missing"))
}
