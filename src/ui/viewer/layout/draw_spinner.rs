use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Align2, Area, Color32, Context, FontId, Order, Spinner, vec2};

use crate::ui::viewer::app::App;

impl App {
    /// Рисует оверлей со спиннером и анимированными точками "Decoding"
    pub(crate) fn draw_spinner(&self, ctx: &Context) {
        Area::new("loading_overlay".into())
            .order(Order::Foreground)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_size(vec2(220.0, 100.0));
                ui.vertical_centered(|ui| {
                    ui.add(Spinner::new().size(44.0));
                    ui.add_space(8.0);

                    let t = ui.input(|i| i.time);
                    let phase = ((t * 6.0).floor() as usize) % 6;

                    // Паттерн для трёх точек справа:
                    // 0: .__   1: .._   2: ...   3: _..   4: __.   5: ___
                    let pattern: [bool; 3] = match phase {
                        0 => [true, false, false],
                        1 => [true, true, false],
                        2 => [true, true, true],
                        3 => [false, true, true],
                        4 => [false, false, true],
                        _ => [false, false, false],
                    };

                    let mut job = LayoutJob::default();
                    let font = FontId::proportional(16.0);
                    let fg = ui
                        .visuals()
                        .widgets
                        .inactive
                        .fg_stroke
                        .color;

                    // слева: всегда три прозрачные точки для выравнивания
                    job.append("...", 0.0, TextFormat { font_id: font.clone(), color: Color32::TRANSPARENT, ..Default::default() });

                    // текст
                    job.append("Decoding", 0.0, TextFormat { font_id: font.clone(), color: fg, ..Default::default() });

                    // справа: анимированные три точки
                    for &visible in &pattern {
                        job.append(".", 0.0, TextFormat { font_id: font.clone(), color: if visible { fg } else { Color32::TRANSPARENT }, ..Default::default() });
                    }

                    ui.label(job);
                });
            });
    }
}
