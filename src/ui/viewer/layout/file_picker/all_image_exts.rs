use image::ImageFormat;
use std::collections::BTreeSet;
use std::sync::OnceLock;

static ALL_IMAGE_EXTS: OnceLock<Vec<&'static str>> = OnceLock::new();

pub(in crate::ui::viewer) fn all_image_exts() -> &'static [&'static str] {
    ALL_IMAGE_EXTS
        .get_or_init(|| {
            let mut set: BTreeSet<&'static str> = BTreeSet::new();

            for fmt in ImageFormat::all() {
                for &ext in fmt.extensions_str() {
                    set.insert(ext);
                }
            }

            set.insert("blp");
            set.insert("psd");

            set.into_iter().collect::<Vec<_>>()
        })
        .as_slice()
}
