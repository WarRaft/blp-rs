use crate::ui::viewer::layout::file_picker::file_pick_input::FilePickInput;
use crate::error::error::BlpError;
use crate::ext::path::ensure_readable::EnsureReadable;
use crate::ui::viewer::app::App;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

impl App {
    pub(crate) fn pick_from_file(&mut self, path: Option<PathBuf>) -> Result<(), BlpError> {
        let Some(path) = path else {
            return Ok(()); // ничего не выбрано
        };

        path.as_path().ensure_readable()?;

        self.picked_file = Some(path.clone());
        self.blp = None;
        self.mip_textures.fill_with(|| None);

        let (tx, rx) = mpsc::sync_channel(1);
        self.decode_rx = Some(rx);
        self.loading = true;

        // поток для декодирования
        thread::spawn(move || {
            let res = FilePickInput::Path(path).decode();
            let _ = tx.send(res);
        });

        Ok(())
    }
}
