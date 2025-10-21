use crate::ui::viewer::app::App;
use crate::ui::viewer::layout::file_picker::all_image_exts::all_image_exts;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const APP: &str = env!("CARGO_PKG_NAME");
const CFG: Option<&str> = Some(stringify!(LastOpenDir));

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LastOpenDir {
    path: Option<PathBuf>,
}

#[inline]
fn platform_desktop() -> Option<PathBuf> {
    directories::UserDirs::new().and_then(|u| u.desktop_dir().map(|p| p.to_path_buf()))
}

#[inline]
fn load_last_open_dir() -> Option<PathBuf> {
    confy::load::<LastOpenDir>(APP, CFG)
        .ok()
        .and_then(|c| c.path)
        .filter(|p| p.is_dir())
}

#[inline]
fn save_last_open_dir(dir: &Path) {
    let _ = confy::store(APP, CFG, &LastOpenDir { path: Some(dir.to_path_buf()) });
}

impl App {
    pub(crate) fn file_dialog_open(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .set_title(self.tr("select-image"))
            .add_filter(self.tr("filter-all-images"), all_image_exts());

        if let Some(dir) = load_last_open_dir()
            .or_else(platform_desktop)
            .or_else(|| std::env::current_dir().ok())
        {
            dlg = dlg.set_directory(dir);
        }

        if let Some(path) = dlg.pick_file() {
            if let Some(parent) = path.parent() {
                save_last_open_dir(parent);
            }
            if let Err(e) = self.pick_from_file(Some(path)) {
                self.error = Some(e);
            }
        }
    }
}
