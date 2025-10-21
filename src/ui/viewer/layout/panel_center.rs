use crate::core::image::MAX_MIPS;
use crate::ui::viewer::app::App;
use eframe::egui::{self, Align, CentralPanel, Frame, Image, Label, Layout, Margin, RichText, ScrollArea, Sense, vec2};

impl App {
    pub(crate) fn draw_panel_center(&mut self, ctx: &egui::Context) {
        CentralPanel::default()
            .frame(Frame::default())
            .show(ctx, |ui| {
                if self.loading {
                    return self.draw_spinner(ctx);
                }

                let any_visible = self
                    .mip_visible
                    .iter()
                    .any(|visible| *visible);
                if !any_visible {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new(self.tr("no-visible-mip-hint")));
                    });
                    return;
                }
                ScrollArea::vertical()
                    .id_salt("right_scroll_mips")
                    .show(ui, |ui| {
                        let spy = ui.spacing().item_spacing.y;

                        ui.add_space(spy * 2.0);
                        let pad_lr: i8 = ui.spacing().item_spacing.x.round() as i8;
                        for i in 0..MAX_MIPS {
                            if !self.mip_visible[i] {
                                continue;
                            }

                            let (w, h) = self
                                .blp
                                .as_ref()
                                .and_then(|b| b.mipmaps.get(i))
                                .map(|m| (m.width, m.height))
                                .unwrap_or((0, 0));

                            let tex_opt = self
                                .mip_textures
                                .get(i)
                                .cloned()
                                .flatten();

                            // внешний горизонтальный паддинг
                            Frame { inner_margin: Margin { left: pad_lr, right: pad_lr, top: 0, bottom: 0 }, ..Default::default() }.show(ui, |ui| {
                                let title = format!("#{i:02} {w}×{h}");

                                // ширина правого текста (моно)
                                let right_w = ui.fonts_mut(|f| {
                                    let style = egui::TextStyle::Monospace.resolve(ui.style());
                                    let galley = f.layout_no_wrap(title.clone(), style, ui.style().visuals.text_color());
                                    galley.size().x
                                });

                                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                                    let spacing = ui.spacing().item_spacing.x;
                                    let left_w = (ui.available_width() - right_w - spacing).max(0.0);

                                    let noim = RichText::new(self.tr("no-image"));

                                    // Левый блок
                                    ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                                        if let Some(tex) = &tex_opt {
                                            let tex_size = tex.size_vec2();
                                            if tex_size.x > 0.0 && tex_size.y > 0.0 && left_w > 0.0 {
                                                let draw_w = left_w.min(tex_size.x);
                                                let draw_h = draw_w * (tex_size.y / tex_size.x);
                                                ui.add(Image::from_texture((tex.id(), tex_size)).fit_to_exact_size(vec2(draw_w, draw_h)));
                                            } else {
                                                ui.label(noim.clone());
                                            }
                                        } else {
                                            ui.label(noim.clone());
                                        }
                                    });

                                    // спейсер
                                    let rem = ui.available_width();
                                    if rem > right_w {
                                        ui.add_space(rem - right_w);
                                    }

                                    // Правый блок
                                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                        ui.add(Label::new(RichText::new(title).monospace()).truncate());
                                    });
                                });
                            });

                            ui.add_space(spy);
                        }

                        // растяжка-строка
                        let _ = ui.allocate_exact_size(vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}
