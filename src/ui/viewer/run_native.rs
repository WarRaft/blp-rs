use crate::error::error::BlpError;
use crate::ui::viewer::app::App;
use eframe::egui::{IconData, ViewportBuilder, vec2};
use eframe::{NativeOptions, Renderer};
use std::path::PathBuf;
use std::sync::Arc;

#[inline]
fn report_error(msg: &str) {
    // stderr (visible if launched from a terminal)
    eprintln!("blp: {msg}");

    // Native dialog (requires `rfd`, which you already include under the `ui` feature)
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("blp - error")
        .set_description(msg)
        .show();
}

pub fn run_native(path: Option<PathBuf>) -> Result<(), BlpError> {
    eframe::run_native(
        "blp",
        NativeOptions {
            persist_window: true, //
            viewport: ViewportBuilder {
                title: Some("blp".to_string()), //
                app_id: Some("org.warraft.blp".to_string()),
                inner_size: Some(vec2(800.0, 680.0)),
                clamp_size_to_monitor_size: Some(true),
                decorations: Some(false),
                resizable: Some(true),
                icon: Some(Arc::new({
                    let img = image::load_from_memory(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png")))?.into_rgba8();
                    let (w, h) = img.dimensions();
                    IconData { rgba: img.into_raw(), width: w, height: h }
                })),
                has_shadow: Some(true),
                ..Default::default()
            },
            renderer: Renderer::Wgpu,
            vsync: true,
            ..Default::default()
        },
        Box::new(move |cc| -> Result<Box<dyn eframe::App>, _> {
            let mut app = App::new(&cc.egui_ctx);
            if let Err(e) = app.pick_from_file(path.clone()) {
                report_error(&format!("Failed to open file: {}", e));
                app.error = Some(e);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| {
        report_error(&format!("Failed to launch UI: {}", err));
        BlpError::new("ui-run-native").with_arg("msg", err.to_string())
    })
}
