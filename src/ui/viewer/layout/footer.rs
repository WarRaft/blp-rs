use crate::ui::i18n::lng_list::LngList;
use crate::ui::i18n::prefs::save_lang;
use crate::ui::viewer::app::App;
use eframe::egui::{Align, Color32, ComboBox, Context, CursorIcon, Frame, Label, Layout, Margin, RichText, ScrollArea, Stroke, TopBottomPanel};

impl App {
    pub(crate) fn draw_footer(&mut self, ctx: &Context) {
        if let Some(err) = &self.error {
            let plain = self.err_text_localized(err);

            TopBottomPanel::bottom("footer_error")
                .resizable(true)
                .show_separator_line(false)
                .frame(Frame {
                    fill: Color32::from_rgba_unmultiplied(18, 8, 12, 230), //
                    stroke: Stroke::new(1.0, Color32::from_rgb(255, 70, 70)),
                    inner_margin: Margin::symmetric(8, 8),
                    outer_margin: Margin { left: 0, right: 0, top: 8, bottom: 0 },
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("⚠ {}", self.tr("error")))
                                .strong()
                                .color(Color32::from_rgb(255, 120, 120)),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .button(self.tr("close"))
                                .on_hover_text(self.tr("close-error-hint"))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.error = None;
                            }
                            if ui
                                .button(self.tr("copy"))
                                .on_hover_text(self.tr("copy-error-hint"))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                ui.ctx()
                                    .copy_text(format!("```\n{}\n```", plain));
                            }
                        });
                    });

                    ui.add_space(6.0);

                    ScrollArea::both()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.add(
                                Label::new(RichText::new(plain).monospace())
                                    .wrap()
                                    .selectable(true),
                            );
                        });
                });
        }

        TopBottomPanel::bottom("footer_menu")
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame {
                inner_margin: Margin::symmetric(8, 8), //
                // Вариант A — мягкий teal
                fill: Color32::from_rgba_unmultiplied(16, 24, 26, 190),
                stroke: Stroke::new(1.0, Color32::from_rgba_unmultiplied(28, 120, 120, 150)),

                ..Default::default()
            })
            .show(ctx, |ui| {
                let ir = ComboBox::from_id_salt("menu_lng")
                    .selected_text(self.lng.name())
                    .show_ui(ui, |ui| {
                        for cand in [LngList::Uk, LngList::Ru, LngList::En, LngList::Zh, LngList::Tc] {
                            let sel = self.lng == cand;
                            if ui
                                .selectable_label(sel, cand.name())
                                .on_hover_cursor(CursorIcon::PointingHand) // курсор в выпаде
                                .clicked()
                                && !sel
                            {
                                self.lng = cand;
                                let _ = save_lang(self.lng);
                            }
                        }
                    });

                // курсор над свернутым комбобоксом
                ir.response
                    .on_hover_cursor(CursorIcon::PointingHand);
            });
    }
}
