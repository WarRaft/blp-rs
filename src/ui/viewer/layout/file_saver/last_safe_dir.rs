use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const APP: &str = env!("CARGO_PKG_NAME");
const CFG: Option<&str> = Some(stringify!(LastSaveDir));

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LastSaveDir {
    dir: Option<PathBuf>,
}

pub fn last_save_dir_load() -> Option<PathBuf> {
    let s: LastSaveDir = confy::load(APP, CFG).unwrap_or_default();
    s.dir
}

pub fn last_save_dir_save(dir: &Path) -> Result<(), confy::ConfyError> {
    let dir = if dir.is_dir() { dir.to_path_buf() } else { dir.to_path_buf() };
    confy::store(APP, CFG, LastSaveDir { dir: Some(dir) })
}
