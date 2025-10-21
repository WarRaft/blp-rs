// scan_patched.rs

#[cfg(test)]
pub mod mod_scan_patched {
    use byteorder::{ByteOrder, LittleEndian};
    use sha1::{Digest, Sha1};
    use std::ffi::OsStr;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    // ---- JPEG header info we care about ----
    #[derive(Debug, Clone)]
    struct JpegHeaderInfo {
        header_end: usize, // first byte of entropy (after SOS segment payload)
        sof_pos: usize,    // offset of SOF marker (FFC0/FFC2)
        sof_marker: u8,    // C0 (baseline) or C2 (progressive)
        sof_h_off: usize,  // offset of Height (big-endian u16) inside whole JPEG buffer
        sof_w_off: usize,  // offset of Width  (big-endian u16) inside whole JPEG buffer
        sof_h: u16,
        sof_w: u16,
    }

    #[derive(Debug)]
    struct MipRow {
        file: PathBuf,
        idx: usize,
        blp_w: u32,
        blp_h: u32,
        sof_w: Option<u16>,
        sof_h: Option<u16>,
        header_len: usize,
        header_sha1: String,
        lcp_to_mip0: usize,
        header_eq_mip0: bool,
        header_eq_mip0_except_hw: bool,
    }

    #[derive(Debug, Default)]
    struct FileSummary {
        file: PathBuf,
        // overall
        mips: usize,
        all_equal_headers: bool,
        all_equal_headers_except_hw: bool,
        // counts
        mismatched_sof_dims: usize, // how many mips where SOF dims != expected mip dims
        // lengths
        header_len_ref: usize,
        // lcp
        lcp_min: usize,
        lcp_max: usize,
        lcp_avg: f64,
    }

    // ---------------- BLP1 (JPEG) lightweight parser ----------------
    #[derive(Debug)]
    struct Blp1Jpeg {
        width_base: u32,
        height_base: u32,
        header: Vec<u8>, // common JPEG header (usually up to/during SOS, but not guaranteed)
        mip_offsets: [u32; 16],
        mip_sizes: [u32; 16],
        data: Vec<u8>, // whole file bytes to slice scan data from
    }

    fn parse_blp1_jpeg(data: Vec<u8>, path: &Path) -> io::Result<Option<Blp1Jpeg>> {
        if data.len() < 4 {
            return Ok(None);
        }
        let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x424C5031 {
            return Ok(None);
        }
        if data.len() < 8 {
            return Ok(None);
        }
        let compression = LittleEndian::read_u32(&data[4..8]);
        if compression != 0 {
            // Not JPEG, skip
            return Ok(None);
        }
        if data.len() < 32 {
            return Ok(None);
        }
        let width = LittleEndian::read_u32(&data[12..16]);
        let height = LittleEndian::read_u32(&data[16..20]);

        // Skip 2 more u32 (8 bytes)
        let mut off = 4 + 4 + 4 + 4 + 4 + 4 + 4;
        if data.len() < off + 16 * 4 + 16 * 4 + 4 {
            return Ok(None);
        }

        let mut mip_offsets = [0u32; 16];
        let mut mip_sizes = [0u32; 16];
        for i in 0..16 {
            mip_offsets[i] = LittleEndian::read_u32(&data[off + i * 4..off + (i + 1) * 4]);
        }
        off += 16 * 4;
        for i in 0..16 {
            mip_sizes[i] = LittleEndian::read_u32(&data[off + i * 4..off + (i + 1) * 4]);
        }
        off += 16 * 4;

        let jpeg_header_size = LittleEndian::read_u32(&data[off..off + 4]) as usize;
        off += 4;
        if data.len() < off + jpeg_header_size {
            eprintln!("⚠️  {}: corrupted jpeg_header_size", path.display());
            return Ok(None);
        }
        let header = data[off..off + jpeg_header_size].to_vec();
        Ok(Some(Blp1Jpeg { width_base: width, height_base: height, header, mip_offsets, mip_sizes, data }))
    }

    // Compute expected mip WxH by level (stop at 1)
    #[inline]
    fn mip_dims(base_w: u32, base_h: u32, level: usize) -> (u32, u32) {
        let mut w = base_w;
        let mut h = base_h;
        for _ in 0..level {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        (w, h)
    }

    // --------------- JPEG marker parsing (up to SOS) ----------------
    fn parse_jpeg_header_info(buf: &[u8]) -> Option<JpegHeaderInfo> {
        if buf.len() < 4 {
            return None;
        }
        // SOI?
        if buf[0] != 0xFF || buf[1] != 0xD8 {
            return None;
        }

        let mut i = 2usize;
        let mut sof_pos = None::<usize>;
        let mut sof_marker = 0u8;
        let mut sof_h_off = 0usize;
        let mut sof_w_off = 0usize;
        let mut sof_h = 0u16;
        let mut sof_w = 0u16;

        while i + 1 < buf.len() {
            if buf[i] != 0xFF {
                return None;
            }
            // Skip fill FFs
            while i < buf.len() && buf[i] == 0xFF {
                i += 1;
            }
            if i >= buf.len() {
                return None;
            }
            let marker = buf[i];
            i += 1;

            match marker {
                0xD8 => { /* SOI again (unlikely) */ }
                0xD9 => {
                    return None;
                } // EOI in header — weird
                0xDA => {
                    // SOS
                    if i + 2 > buf.len() {
                        return None;
                    }
                    let ls = u16::from_be_bytes([buf[i], buf[i + 1]]) as usize;
                    i += 2;
                    if i + (ls - 2) > buf.len() {
                        return None;
                    }
                    i += ls - 2;
                    let header_end = i; // first entropy byte
                    return Some(JpegHeaderInfo { header_end, sof_pos: sof_pos.unwrap_or(0), sof_marker, sof_h_off, sof_w_off, sof_h, sof_w });
                }
                0xC0 | 0xC2 => {
                    // SOF0 / SOF2
                    if i + 2 > buf.len() {
                        return None;
                    }
                    let lh = u16::from_be_bytes([buf[i], buf[i + 1]]) as usize;
                    let seg_start = i - 2; // includes 0xFF 0xC0 and length
                    i += 2;
                    if i + (lh - 2) > buf.len() {
                        return None;
                    }
                    if lh < 8 {
                        return None;
                    }

                    let _p = buf[i]; // precision (expect 8)
                    let yh = buf[i + 1];
                    let yl = buf[i + 2];
                    let xh = buf[i + 3];
                    let xl = buf[i + 4];
                    let h = u16::from_be_bytes([yh, yl]);
                    let w = u16::from_be_bytes([xh, xl]);

                    sof_pos = Some(seg_start);
                    sof_marker = marker;
                    sof_h_off = i + 1;
                    sof_w_off = i + 3;
                    sof_h = h;
                    sof_w = w;

                    i += lh - 2; // skip rest of segment
                }
                0xD0..=0xD7 => {
                    // RSTn — shouldn't appear before SOS; ignore defensively
                }
                _ => {
                    // Generic segment with length
                    if i + 2 > buf.len() {
                        return None;
                    }
                    let l = u16::from_be_bytes([buf[i], buf[i + 1]]) as usize;
                    i += 2;
                    if i + (l - 2) > buf.len() {
                        return None;
                    }
                    i += l - 2;
                }
            }
        }
        None
    }

    #[inline]
    fn sha1_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let out = hasher.finalize();
        hex::encode(out)
    }

    // Compare header equality "except for 4 H/W bytes"
    fn header_eq_except_hw(a: &[u8], b: &[u8], hi: &JpegHeaderInfo, hj: &JpegHeaderInfo) -> bool {
        if a.len() != b.len() {
            return false;
        }
        if hi.sof_w_off == 0 || hj.sof_w_off == 0 {
            return a == b;
        }
        for (i, (&ba, &bb)) in a.iter().zip(b.iter()).enumerate() {
            if i == hi.sof_h_off || i == hi.sof_h_off + 1 || i == hi.sof_w_off || i == hi.sof_w_off + 1 {
                continue;
            }
            if ba != bb {
                return false;
            }
        }
        true
    }

    // ------------------ public entrypoint ------------------
    pub fn run(input_root: &Path, out_csv: &Path) {
        let mut rows: Vec<MipRow> = Vec::new();
        let mut file_summaries: Vec<FileSummary> = Vec::new();

        for entry in WalkDir::new(input_root)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path
                .extension()
                .and_then(OsStr::to_str)
                .map(|e| e.eq_ignore_ascii_case("blp"))
                != Some(true)
            {
                continue;
            }

            let data = match fs::read(path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("❌ read {}: {e}", path.display());
                    continue;
                }
            };

            let blp = match parse_blp1_jpeg(data, path) {
                Ok(Some(blp)) => blp,
                Ok(None) => continue, // Not BLP1/JPEG
                Err(e) => {
                    eprintln!("❌ parse {}: {e}", path.display());
                    continue;
                }
            };

            // Build full JPEGs: header + scan
            let mut fulls: Vec<Vec<u8>> = Vec::new();
            let mut mip_dims_vec: Vec<(u32, u32)> = Vec::new();
            for i in 0..16 {
                let sz = blp.mip_sizes[i] as usize;
                let off = blp.mip_offsets[i] as usize;
                if sz == 0 {
                    break;
                }
                if off + sz > blp.data.len() {
                    eprintln!("⚠️  {}: bad offset/size at mip {}", path.display(), i);
                    break;
                }
                let (mw, mh) = mip_dims(blp.width_base, blp.height_base, i);
                mip_dims_vec.push((mw, mh));

                let mut full = Vec::with_capacity(blp.header.len() + sz);
                full.extend_from_slice(&blp.header);
                full.extend_from_slice(&blp.data[off..off + sz]);
                fulls.push(full);
            }
            if fulls.is_empty() {
                continue;
            }

            // Parse headers
            let mut headers: Vec<Vec<u8>> = Vec::new();
            let mut infos: Vec<JpegHeaderInfo> = Vec::new();
            let mut any_parse_fail = false;
            for (idx, full) in fulls.iter().enumerate() {
                if let Some(info) = parse_jpeg_header_info(full) {
                    headers.push(full[..info.header_end].to_vec());
                    infos.push(info);
                } else {
                    eprintln!("⚠️  {}: JPEG parse failed at mip {}", path.display(), idx);
                    any_parse_fail = true;
                    break;
                }
            }
            if any_parse_fail {
                continue;
            }

            // compare headers to mip0
            let ref_header = &headers[0];
            let ref_info = &infos[0];

            let mut all_equal = true;
            let mut all_equal_except_hw = true;
            let mut lcps: Vec<usize> = Vec::with_capacity(headers.len());
            lcps.push(ref_header.len()); // with itself

            for (idx, hdr) in headers.iter().enumerate().skip(1) {
                if hdr != ref_header {
                    all_equal = false;
                }
                let eq_except_hw = header_eq_except_hw(hdr, ref_header, &infos[idx], ref_info);
                if !eq_except_hw {
                    all_equal_except_hw = false;
                }

                // lcp across full JPEGs (just for curiosity)
                let l = {
                    let a = &fulls[0];
                    let b = &fulls[idx];
                    let min_len = a.len().min(b.len());
                    let mut j = 0usize;
                    while j < min_len && a[j] == b[j] {
                        j += 1;
                    }
                    j
                };
                lcps.push(l);
            }

            // collect per-mip rows
            for (i, (hdr, info)) in headers
                .iter()
                .zip(infos.iter())
                .enumerate()
            {
                let (mw, mh) = mip_dims_vec[i];
                let row = MipRow { file: path.to_path_buf(), idx: i, blp_w: mw, blp_h: mh, sof_w: Some(info.sof_w), sof_h: Some(info.sof_h), header_len: hdr.len(), header_sha1: sha1_hex(hdr), lcp_to_mip0: lcps[i], header_eq_mip0: hdr == ref_header, header_eq_mip0_except_hw: header_eq_except_hw(hdr, ref_header, info, ref_info) };
                rows.push(row);
            }

            // summary
            let mut mismatched = 0usize;
            for (i, info) in infos.iter().enumerate() {
                let (mw, mh) = mip_dims_vec[i];
                if info.sof_w as u32 != mw || info.sof_h as u32 != mh {
                    mismatched += 1;
                }
            }
            let lcp_min = *lcps.iter().min().unwrap_or(&0);
            let lcp_max = *lcps.iter().max().unwrap_or(&0);
            let lcp_avg = if !lcps.is_empty() { (lcps.iter().sum::<usize>() as f64) / (lcps.len() as f64) } else { 0.0 };

            file_summaries.push(FileSummary {
                file: path.to_path_buf(),
                mips: headers.len(),
                all_equal_headers: all_equal,
                all_equal_headers_except_hw: all_equal_except_hw, // фикс имени поля
                mismatched_sof_dims: mismatched,
                header_len_ref: ref_header.len(),
                lcp_min,
                lcp_max,
                lcp_avg,
            });
        }

        // ---- Print concise report ----
        let mut total_files = 0usize;
        let mut jpeg_with_all_equal = 0usize;
        let mut jpeg_all_equal_except_hw = 0usize;
        let mut files_with_mismatch_dims = 0usize;

        for s in &file_summaries {
            total_files += 1;
            if s.all_equal_headers {
                jpeg_with_all_equal += 1;
            }
            if s.all_equal_headers_except_hw {
                jpeg_all_equal_except_hw += 1;
            }
            if s.mismatched_sof_dims > 0 {
                files_with_mismatch_dims += 1;
            }
        }

        println!("\n===== BLP1/JPEG header patch detection =====");
        println!("Scanned files:            {}", total_files);
        println!("All headers equal:        {} files", jpeg_with_all_equal);
        println!("Equal except H/W bytes:   {} files", jpeg_all_equal_except_hw);
        println!("SOF dims mismatch (any):  {} files", files_with_mismatch_dims);

        // ---- Write CSV ----
        // по умолчанию положим csv рядом с проектом (путь берётся из теста)
        if let Err(e) = write_csv(out_csv, &rows) {
            eprintln!("❌ write csv {}: {e}", out_csv.display());
        } else {
            println!("CSV saved to {}", out_csv.display());
        }
    }

    fn write_csv(path: &Path, rows: &[crate_row_alias::RowAlias]) -> io::Result<()> {
        // минимальный CSV без внешних зависимостей
        let mut out = String::new();
        out.push_str("file,mip,blp_w,blp_h,sof_w,sof_h,header_len,header_sha1,lcp_to_mip0,header_eq_mip0,header_eq_mip0_except_hw\n");
        for r in rows {
            let sof_w = r
                .sof_w
                .map(|v| v.to_string())
                .unwrap_or_default();
            let sof_h = r
                .sof_h
                .map(|v| v.to_string())
                .unwrap_or_default();
            // quote file path because it may contain commas
            out.push('"');
            out.push_str(&r.file.to_string_lossy());
            out.push('"');
            out.push(',');
            out.push_str(&format!("{},{},{},{},{},{},\"{}\",{},{},{}\n", r.idx, r.blp_w, r.blp_h, sof_w, sof_h, r.header_len, r.header_sha1, r.lcp_to_mip0, r.header_eq_mip0 as u8, r.header_eq_mip0_except_hw as u8));
        }
        fs::write(path, out)
    }

    // alias to keep write_csv signature stable
    mod crate_row_alias {
        use super::MipRow;
        pub type RowAlias = MipRow;
    }

    // ---------------------- TEST HOOK ----------------------
    // Запускается командой: cargo test -- --nocapture
    #[test]
    fn scan_patched() {
        // возьми папку с распакованными BLP из env, иначе — дефолт из твоего примера
        let input = std::env::var("BLP_SCAN_DIR").unwrap_or_else(|_| "/Users/nazarpunk/IdeaProjects/War3.mpq/extract".to_string());
        // куда положить csv (можно также управлять через env)
        let out_csv = std::env::var("BLP_SCAN_OUT").unwrap_or_else(|_| "test-data/scan_patched.csv".to_string());

        // создадим папку назначения, если её нет
        if let Some(parent) = Path::new(&out_csv).parent() {
            let _ = fs::create_dir_all(parent);
        }

        run(Path::new(&input), Path::new(&out_csv));
    }
}
