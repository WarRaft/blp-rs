use image::ImageFormat;
use std::collections::BTreeSet;
use std::sync::OnceLock;

static ALL_IMAGE_EXTS: OnceLock<Vec<&'static str>> = OnceLock::new();

pub(in crate::ui::viewer) fn all_image_exts() -> &'static [&'static str] {
    ALL_IMAGE_EXTS
        .get_or_init(|| {
            let mut set: BTreeSet<&'static str> = BTreeSet::new();

            // Все форматы, известные crate `image` (зависят от включённых фич)
            for fmt in ImageFormat::all() {
                for &ext in fmt.extensions_str() {
                    set.insert(ext);
                }
            }

            // Плюс наши кастомные
            set.insert("blp");

            // Если хочешь явно добавить редкие, которые у тебя точно поддержаны, раскомментируй:
            // set.insert("dds");
            // set.insert("tga");
            // set.insert("qoi");
            // set.insert("avif");

            set.into_iter().collect::<Vec<_>>()
        })
        .as_slice()
}
