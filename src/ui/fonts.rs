use eframe::egui::{Context, FontFamily};
use eframe::epaint::text::{FontData, FontDefinitions};
use std::sync::Arc;

// === Direct TTF embedding from repository ===
// These constants embed the raw font files into the binary at compile time.
// Paths are relative to the crate root, which makes this approach reliable
// across different build environments (cargo, build.rs, etc.).
const JB_MONO_REG: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/JetBrainsMono-Regular.ttf"));
const WENKAI_SC_REG: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/LXGWWenKaiMono-Regular.ttf"));
const WENKAI_TC_REG: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/LXGWWenKaiMonoTC-Regular.ttf"));

/// Insert a static font (loaded via `include_bytes!`) into the given `FontDefinitions`.
///
/// - `key`: The name under which the font will be registered.
/// - `bytes`: The raw font data (must be `'static`, since it's embedded).
#[inline]
fn insert_static(defs: &mut FontDefinitions, key: &str, bytes: &'static [u8]) {
    defs.font_data
        .insert(key.to_string(), Arc::from(FontData::from_static(bytes)));
}

/// Move a registered font to the **front** of a given family (`Proportional` or `Monospace`).
///
/// This ensures the font is used with highest priority for that family.
/// If the font is not present in `font_data`, nothing happens.
fn push_front_unique(defs: &mut FontDefinitions, family: FontFamily, name: &str) {
    if !defs.font_data.contains_key(name) {
        return;
    }
    if let Some(list) = defs.families.get_mut(&family) {
        if let Some(pos) = list.iter().position(|s| s == name) {
            list.remove(pos);
        }
        list.insert(0, name.to_string());
    }
}

/// Move a registered font to the **front** of *both* families (`Proportional` and `Monospace`).
///
/// Safe: If the font key is missing from `font_data`, nothing happens.
fn push_front_both(defs: &mut FontDefinitions, name: &str) {
    push_front_unique(defs, FontFamily::Proportional, name);
    push_front_unique(defs, FontFamily::Monospace, name);
}

/// Install bundled fonts (Regular-only) into the `egui` context.
///
/// ### Priority order
/// The intended fallback chain is:
/// - **JetBrains Mono** → primary Latin/Cyrillic font
/// - **WenKai TC** → Traditional Chinese fallback
/// - **WenKai SC** → Simplified Chinese fallback
///
/// Implementation detail:  
/// `push_front_*` inserts a font at the **front** of the list, so we call
/// them in *reverse* order (SC → TC → JB) to produce the desired priority chain.
pub fn install_fonts(ctx: &Context) {
    let mut defs = FontDefinitions::default();

    // 1) Register embedded fonts
    insert_static(&mut defs, "JetBrainsMono-Regular", JB_MONO_REG);
    insert_static(&mut defs, "LXGWWenKaiMonoTC-Regular", WENKAI_TC_REG);
    insert_static(&mut defs, "LXGWWenKaiMono-Regular", WENKAI_SC_REG);

    // 2) Adjust fallback order: SC → TC → JB
    push_front_both(&mut defs, "LXGWWenKaiMono-Regular"); // SC
    push_front_both(&mut defs, "LXGWWenKaiMonoTC-Regular"); // TC
    push_front_both(&mut defs, "JetBrainsMono-Regular"); // JB (final priority: JB → TC → SC)

    // 3) Apply fonts to the `egui` context
    ctx.set_fonts(defs);
}
