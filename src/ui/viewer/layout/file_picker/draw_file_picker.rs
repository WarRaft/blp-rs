use crate::ext::path::to_abs_string_with_macros::PathMacrosExt;
use crate::flargs;
use crate::ui::i18n::shortcut::platform_cmd_shortcut;
use crate::ui::viewer::app::App;
use crate::ui::viewer::layout::file_picker::hotkey_pressed::hotkey_pressed;
use crate::ui::widget::text_edit_ex::TextEditLikeButtonChain;
use eframe::egui::text::{LayoutJob, TextWrapping};
use eframe::egui::{Align, Button, Color32, Context, CornerRadius, CursorIcon, Frame, Galley, Key, Layout, Margin, Sense, Stroke, StrokeKind, TextEdit, TextFormat, TopBottomPanel, pos2, vec2};

impl App {
    pub(crate) fn draw_file_picker(&mut self, ctx: &Context) {
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(path) = f.path {
                if let Err(e) = self.pick_from_file(Some(path)) {
                    self.error = Some(e);
                }
            }
        }

        let open_hotkey = hotkey_pressed(ctx, Key::O);
        let paste_hotkey = hotkey_pressed(ctx, Key::V);

        let style = ctx.style();
        let spacing = &style.spacing;
        let gap = spacing.item_spacing.x;
        let gap_i = gap as i8;

        let mut click_select = false;
        let mut click_paste = false;

        TopBottomPanel::top("file_picker_bar")
            .show_separator_line(false)
            .frame(Frame {
                fill: Color32::TRANSPARENT, //
                stroke: Stroke::NONE,
                outer_margin: Margin { left: gap_i, right: gap_i, top: gap_i, bottom: 0 },
                inner_margin: Margin::same(2),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.add_enabled_ui(!self.loading, |ui| {
                        // Open
                        if ui
                            .add(Button::new(self.tr("open")))
                            .on_hover_text(self.tr_args("open-hint", &flargs!(shortcut = platform_cmd_shortcut("O"))))
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            click_select = true;
                        }

                        ui.add_space(gap);

                        // Paste
                        if ui
                            .add(Button::new(self.tr("paste")))
                            .on_hover_text(self.tr_args("paste-hint", &flargs!(shortcut = platform_cmd_shortcut("V"))))
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            click_paste = true;
                        }
                    });

                    ui.add_space(gap);

                    let row_h = ui.spacing().interact_size.y;
                    let w = ui.available_width();

                    if let Some(path) = self.picked_file.clone() {
                        let mut s = path.to_abs_string_with_macros();
                        ui.add_sized(
                            [w, row_h],
                            TextEdit::singleline(&mut s)
                                .cursor_at_end(true)
                                .desired_width(w)
                                .like_button(),
                        );
                    } else {
                        use std::sync::Arc;
                        let s = self.tr(if self.blp.is_some() { "pasted-image" } else { "drop-hint" });
                        let style = ui.style().clone();
                        let spacing = style.spacing.clone();
                        let pad = spacing.button_padding;

                        // --- КУРСИВНЫЙ GALLEY ---
                        let galley: Arc<Galley> = ui.fonts_mut(|fonts| {
                            let mut job = LayoutJob::single_section(
                                s.to_owned(),
                                TextFormat {
                                    color: style.visuals.text_color(), // базовый цвет, прозрачность ниже
                                    ..Default::default()
                                },
                            );
                            job.wrap = TextWrapping::default(); // без переноса (аналог layout_no_wrap)
                            fonts.layout_job(job)
                        });

                        let w = ui.available_width();
                        let size = vec2(w, galley.size().y + pad.y * 2.0);
                        let (rect, _) = ui.allocate_exact_size(size, Sense::click());

                        ui.painter().rect(
                            rect,
                            CornerRadius::same(0), //
                            Color32::from_rgba_unmultiplied(8, 32, 44, 192),
                            style.visuals.widgets.inactive.bg_stroke,
                            StrokeKind::Inside,
                        );

                        let text_pos = pos2(rect.min.x + pad.x, rect.min.y + pad.y);

                        // чуть приглушим текст
                        let color = style
                            .visuals
                            .text_color()
                            .linear_multiply(0.85);
                        ui.painter()
                            .galley(text_pos, galley, color);
                    }
                });
            });

        // Действия
        if open_hotkey || click_select {
            self.file_dialog_open();
        }
        if click_paste || paste_hotkey {
            if let Err(e) = self.pick_from_clipboard() {
                self.error = Some(e);
            }
        }
    }
}
