use crate::ui::viewer::layout::file_picker::file_pick_input::FilePickInput;
use crate::error::error::BlpError;
use crate::ui::viewer::app::App;

impl App {
    pub(crate) fn pick_from_clipboard(&mut self) -> Result<(), BlpError> {
        // ---------- macOS: file:// из Finder ----------
        #[cfg(target_os = "macos")]
        {
            use crate::ui::viewer::layout::file_picker::macos_paste_event::pasteboard_file_path;

            if let Some(path) = pasteboard_file_path().filter(|p| p.is_file()) {
                self.pick_from_file(Some(path))?;
                return Ok(());
            }
        }

        // ---------- bitmap через arboard ----------
        use arboard::Clipboard;
        use image::{DynamicImage, ImageFormat};
        use std::io::Cursor;
        use std::sync::mpsc;
        use std::thread;

        // init буфера обмена
        let mut cb = Clipboard::new().map_err(|e| BlpError::new("error-clipboard-init-failed").push_std(e))?;

        // получаем RGBA-данные из буфера
        let img = cb
            .get_image()
            .map_err(|e| BlpError::new("error-clipboard-no-image").push_std(e))?;

        let w = img.width as u32;
        let h = img.height as u32;
        #[allow(unused_mut)]
        let mut rgba = img.bytes.into_owned();

        // Windows: BGRA → RGBA
        #[cfg(target_os = "windows")]
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        // собираем RgbaImage (проверяем валидность буфера)
        let rgba_img = image::RgbaImage::from_raw(w, h, rgba).ok_or_else(|| {
            BlpError::new("error-clipboard-invalid-image-buffer")
                .with_arg("width", w)
                .with_arg("height", h)
        })?;
        let dyn_img = DynamicImage::ImageRgba8(rgba_img);

        // кодируем во временный PNG (твой декодер ест png/jpg/…)
        let mut buf = Vec::new();
        dyn_img
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| BlpError::new("error-clipboard-encode-png-failed").push_std(e))?;

        // Сброс состояния + запуск декодера
        self.picked_file = None;
        self.error = None;
        self.blp = None;
        self.mip_textures.fill_with(|| None);

        let (tx, rx) = mpsc::sync_channel(1);
        self.decode_rx = Some(rx);
        self.loading = true;

        thread::spawn(move || {
            let res = FilePickInput::Bytes(buf).decode();
            let _ = tx.send(res);
        });

        Ok(())
    }
}
