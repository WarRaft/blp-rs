use crate::error::error::BlpError;
use std::{fs, io, io::Read, path::Path};

/// Расширение для [`Path`] с проверкой и чтением файла.
pub trait EnsureReadable {
    /// Проверить, что путь указывает на **читаемый обычный файл**.
    /// - Ошибки точные: файл не найден / нет прав / это каталог / не обычный файл / битый путь / ошибка метаданных.
    fn ensure_readable(&self) -> Result<(), BlpError>;

    /// Прочитать содержимое файла (одно открытие).
    /// Возвращает подробные ошибки открытия/чтения.
    fn read_all(&self) -> Result<Vec<u8>, BlpError>;
}

impl EnsureReadable for Path {
    fn ensure_readable(&self) -> Result<(), BlpError> {
        // Проверяем метаданные (без открытия) — различаем типы объекта и базовые причины.
        match fs::symlink_metadata(self) {
            Ok(meta) => {
                if meta.is_dir() {
                    return Err(BlpError::new("error-path-is-directory").with_arg("path", self.to_string_lossy().into_owned()));
                }
                if !meta.is_file() {
                    return Err(BlpError::new("error-path-not-regular-file").with_arg("path", self.to_string_lossy().into_owned()));
                }
                Ok(())
            }
            Err(e) => {
                use io::ErrorKind::*;
                let key = match e.kind() {
                    NotFound => "error-file-not-found",
                    PermissionDenied => "error-permission-denied",
                    InvalidInput => "error-invalid-path",
                    _ => "error-io-metadata",
                };
                Err(BlpError::new(key)
                    .with_arg("path", self.to_string_lossy().into_owned())
                    .with_arg("msg", e.to_string())
                    .push_std(e))
            }
        }
    }

    fn read_all(&self) -> Result<Vec<u8>, BlpError> {
        // Открываем (это и проверка прав/существования, и одно открытие).
        let mut file = fs::File::open(self).map_err(|e| {
            use io::ErrorKind::*;
            let key = match e.kind() {
                NotFound => "error-file-not-found",
                PermissionDenied => "error-permission-denied",
                InvalidInput => "error-invalid-path",
                _ => "error-io-open-file",
            };
            BlpError::new(key)
                .with_arg("path", self.to_string_lossy().into_owned())
                .with_arg("msg", e.to_string())
                .push_std(e)
        })?;

        // Читаем.
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| {
                BlpError::new("error-io-read-file")
                    .with_arg("path", self.to_string_lossy().into_owned())
                    .with_arg("msg", e.to_string())
                    .push_std(e)
            })?;

        Ok(buf)
    }
}
