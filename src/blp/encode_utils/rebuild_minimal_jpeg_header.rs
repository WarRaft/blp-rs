use crate::blp::encode_utils::read_be_u16;
use crate::error::error::BlpError;

/// Rebuild a minimal JPEG header that keeps SOI, non-APP/COM markers and the first SOF + SOS.
#[inline]
pub fn rebuild_minimal_jpeg_header(header: &[u8]) -> Result<Vec<u8>, BlpError> {
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
