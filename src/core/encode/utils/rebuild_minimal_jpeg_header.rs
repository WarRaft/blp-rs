use crate::core::encode::utils::read_be_u16::read_be_u16;
use crate::error::error::BlpError;

/// Принимает JPEG-заголовок от SOI до SOS (включительно) и
/// возвращает минимальный заголовок: SOI + [без APPn/COM] + первый SOF + SOS.
/// Оставляет standalone-маркеры (TEM, RST0..7).
#[inline]
pub fn rebuild_minimal_jpeg_header(header: &[u8]) -> Result<Vec<u8>, BlpError> {
    // Проверка SOI (FFD8)
    if header.len() < 4 || header[0] != 0xFF || header[1] != 0xD8 {
        return Err(BlpError::new("jpeg.bad_soi"));
    }

    let mut pos = 2usize;
    let mut others: Vec<(usize, usize)> = Vec::new();
    let mut sof_seg: Option<(usize, usize)> = None;
    let mut sos_seg: Option<(usize, usize)> = None;

    while pos < header.len() {
        // пропускаем fill-байты 0xFF
        while pos < header.len() && header[pos] == 0xFF {
            pos += 1;
        }
        if pos >= header.len() {
            break;
        }

        let id = header[pos];
        let start = pos - 1; // указывает на 0xFF
        pos += 1;

        // Stand-alone маркеры без длины: TEM (0x01) и RST0..RST7 (0xD0..0xD7)
        if id == 0x01 || (0xD0..=0xD7).contains(&id) {
            // их оставляем как есть
            others.push((start, pos));
            continue;
        }

        // Маркеры с длиной (2 байта BE сразу после id)
        if pos + 2 > header.len() {
            return Err(BlpError::new("jpeg.seg_len"));
        }
        let seg_len = read_be_u16(&header[pos..pos + 2])? as usize;
        let end = pos + seg_len;
        if end > header.len() {
            return Err(BlpError::new("jpeg.seg_trunc"));
        }

        // Классификация маркера (инлайн вместо is_app/is_com/is_sof)
        if id == 0xDA {
            // SOS — последний в заголовке, включаем и выходим
            sos_seg = Some((start, end));
            break;
        } else if (0xE0..=0xEF).contains(&id) || id == 0xFE {
            // APPn (E0..EF) и COM (FE) — выкидываем
            // ничего не добавляем в others
        } else if (0xC0..=0xCF).contains(&id) && id != 0xC4 && id != 0xC8 {
            // SOF* (кроме DHT=C4 и JPG=C8): берём только первый
            if sof_seg.is_none() {
                sof_seg = Some((start, end));
            }
        } else {
            // Прочие сегменты — оставляем
            others.push((start, end));
        }

        pos = end;
    }

    let (sos_s, sos_e) = sos_seg.ok_or_else(|| BlpError::new("jpeg.sos_missing"))?;
    let (sof_s, sof_e) = sof_seg.ok_or_else(|| BlpError::new("jpeg.sof_missing"))?;

    // Сборка минимального заголовка
    let mut out = Vec::with_capacity(header.len());
    out.extend_from_slice(&header[..2]); // SOI (FF D8)
    for (s, e) in others {
        out.extend_from_slice(&header[s..e]);
    }
    out.extend_from_slice(&header[sof_s..sof_e]); // первый SOF
    out.extend_from_slice(&header[sos_s..sos_e]); // SOS
    Ok(out)
}
