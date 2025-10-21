use serde::{Deserialize, Serialize};

const APP: &str = env!("CARGO_PKG_NAME");
const CFG: Option<&str> = Some(stringify!(ExportQuality));

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportQualityPrefs {
    pub val: u8,
}

impl Default for ExportQualityPrefs {
    fn default() -> Self {
        Self { val: 100 }
    }
}

pub fn export_quality_load() -> u8 {
    confy::load::<ExportQualityPrefs>(APP, CFG)
        .unwrap_or_default()
        .val
        .clamp(0, 100)
}

pub fn export_quality_save(val: u8) -> Result<(), confy::ConfyError> {
    confy::store(APP, CFG, ExportQualityPrefs { val })
}
