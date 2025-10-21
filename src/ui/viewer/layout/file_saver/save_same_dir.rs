use serde::{Deserialize, Serialize};

const APP: &str = env!("CARGO_PKG_NAME");
const CFG: Option<&str> = Some(stringify!(SaveSameDir));

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct SaveSameDirPrefs {
    pub val: bool,
}

pub fn save_same_dir_load() -> bool {
    confy::load::<SaveSameDirPrefs>(APP, CFG)
        .unwrap_or_default()
        .val
}

pub fn save_same_dir_save(val: bool) -> Result<(), confy::ConfyError> {
    confy::store(APP, CFG, SaveSameDirPrefs { val })
}
