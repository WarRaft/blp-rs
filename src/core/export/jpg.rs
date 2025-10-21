use crate::core::image::ImageBlp;
use crate::core::mipmap::Mipmap;
use crate::core::types::TextureType;
use crate::error::error::BlpError;
use std::fs;
use std::path::Path;

impl ImageBlp {
    /// Экспортирует данный мип как "сырой" JPEG:
    /// склеивает общий JPEG header из файла с хвостом этого мипа и записывает в out_path.
    /// Требуется исходный буфер `buf` с .blp данными (тот же, что парсили).
    pub fn export_jpg(&self, mip: &Mipmap, buf: &[u8], out_path: &Path) -> Result<(), BlpError> {
        // Подготовим директорию
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        // Этот метод имеет смысл только для JPEG-BLP
        if self.texture_type != TextureType::JPEG {
            return Err(BlpError::new("export-jpg.not-jpeg"));
        }

        // Общий header
        let h_off = self.header_offset;
        let h_len = self.header_length;
        if h_len == 0 || h_off.checked_add(h_len).is_none() || h_off + h_len > buf.len() {
            return Err(BlpError::new("export-jpg.header.oob")
                .with_arg("offset", h_off as u32)
                .with_arg("length", h_len as u32)
                .with_arg("buf_len", buf.len() as u32));
        }
        let header_bytes = &buf[h_off..h_off + h_len];

        // Хвост выбранного мипа
        let off = mip.offset;
        let len = mip.length;
        if len == 0 || off.checked_add(len).is_none() || off + len > buf.len() {
            return Err(BlpError::new("export-jpg.mip.oob")
                .with_arg("offset", off as u32)
                .with_arg("length", len as u32)
                .with_arg("buf_len", buf.len() as u32));
        }
        let tail = &buf[off..off + len];

        // Склейка [header][tail] и запись
        let mut full = Vec::with_capacity(header_bytes.len() + tail.len());
        full.extend_from_slice(header_bytes);
        full.extend_from_slice(tail);

        fs::write(out_path, &full)?;
        Ok(())
    }
}
