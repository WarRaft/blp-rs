use crate::ext::path::to_abs_string_with_macros::PathMacrosExt;
use crate::ui::viewer::app::App;
use crate::ui::viewer::layout::file_saver::last_safe_dir::{last_save_dir_load, last_save_dir_save};
use std::path::PathBuf;

/// Предпросмотр того, куда именно полетит файл при сохранении.
pub enum SavePreview {
    /// Сохраним напрямую (без диалога) по указанному пути.
    Direct(PathBuf),
    /// Откроем диалог. Укажем стартовую папку (если знаем) и предложенное имя.
    Dialog { start_dir: Option<PathBuf>, name: String },
}

/// ".png" → "png"
fn trim_dot(ext: &str) -> &str {
    ext.strip_prefix('.').unwrap_or(ext)
}

/// Гарантирует расширение у имени файла.
fn ensure_ext(name: &str, ext: &str) -> String {
    let ext = trim_dot(ext);
    if name
        .rsplit('.')
        .next()
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
    {
        name.to_owned()
    } else {
        format!("{name}.{}", ext)
    }
}

/// Если у Path нет нужного расширения — добавляем (только если пусто).
fn ensure_path_has_ext(mut p: PathBuf, ext: &str) -> PathBuf {
    let ext = trim_dot(ext);
    let has = p
        .extension()
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false);
    if !has && p.extension().is_none() {
        p.set_extension(ext);
    }
    p
}

impl App {
    /// Предпросмотр конечного пути (для тултипа/иконки).
    pub(crate) fn preview_save_path(&self, default_name: &str, ext: &str) -> SavePreview {
        let name = ensure_ext(default_name, ext);

        if self.save_same_dir && self.picked_file.is_some() {
            if let Some(src) = self.picked_file.as_ref() {
                if let Some(parent) = src.parent() {
                    return SavePreview::Direct(parent.join(&name));
                }
            }
            // если «рядом» невозможно — пойдём в диалог
        }

        // стартовая папка: confy → родитель picked_file → unknown
        let start_dir = last_save_dir_load().or_else(|| {
            self.picked_file
                .as_ref()
                .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        });

        SavePreview::Dialog { start_dir, name }
    }

    /// Формирует локализованный тултип по превью (локализация через self.tr).
    pub(crate) fn save_preview_tooltip(&self, preview: &SavePreview) -> String {
        match preview {
            SavePreview::Direct(p) => {
                format!("{}\n{}", self.tr("save-tooltip-direct"), p.to_abs_string_with_macros())
            }
            SavePreview::Dialog { start_dir, name } => {
                let dir = start_dir
                    .as_ref()
                    .map(|d| d.to_abs_string_with_macros())
                    .unwrap_or_else(|| {
                        self.tr("save-tooltip-dialog-dir-unknown")
                            .to_owned()
                    });
                format!("{}\n{} {}\n{} {}", self.tr("save-tooltip-dialog"), self.tr("save-tooltip-name"), name, self.tr("save-tooltip-folder"), dir)
            }
        }
    }

    /// Реальное получение пути: учитывает переключатель и запоминает папку.
    pub(crate) fn pick_save_path(&mut self, default_name: &str, ext: &str, desc: String) -> Option<PathBuf> {
        // 1) «рядом», если можем
        if self.save_same_dir && self.picked_file.is_some() {
            if let Some(src) = self.picked_file.as_ref() {
                if let Some(parent) = src.parent() {
                    let file_name = ensure_ext(default_name, ext);
                    return Some(parent.join(file_name));
                }
            }
        }

        // 2) диалог
        let mut dlg = rfd::FileDialog::new()
            .set_file_name(ensure_ext(default_name, ext))
            .add_filter(desc, &[trim_dot(ext)]);

        if let Some(dir) = last_save_dir_load().or_else(|| {
            self.picked_file
                .as_ref()
                .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        }) {
            dlg = dlg.set_directory(dir);
        }

        let path = dlg.save_file()?;
        if let Some(parent) = path.parent() {
            let _ = last_save_dir_save(parent); // best-effort
        }
        Some(ensure_path_has_ext(path, ext))
    }
}
