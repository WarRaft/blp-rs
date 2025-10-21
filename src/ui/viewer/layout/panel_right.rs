use crate::core::image::MAX_MIPS;
use crate::ui::viewer::app::App;
use eframe::egui::{Button, Context, CursorIcon, Frame, Margin, Response, RichText, ScrollArea, Sense, SidePanel, TextStyle, Ui, vec2};

impl App {
    pub(crate) fn draw_panel_right(&mut self, ctx: &Context) {
        SidePanel::right("right_mips")
            .resizable(false)
            .exact_width(180.0)
            .show_separator_line(false)
            .frame(Frame { inner_margin: Margin::same(0), ..Default::default() })
            .show(ctx, |ui| {
                let sp = ui.spacing();
                let spx_f = sp.item_spacing.x;
                let spy_f = sp.item_spacing.y;
                let spx_i = spx_f.round() as i8;

                ScrollArea::vertical()
                    .id_salt("left_scroll_mips")
                    .show(ui, |ui| {
                        Frame { inner_margin: Margin { left: spx_i, right: spx_i, top: 0, bottom: 0 }, ..Default::default() }.show(ui, |ui| {
                            ui.add_space(spy_f * 2.0);
                            ui.add_enabled_ui(!self.loading, |ui| {
                                for i in 0..MAX_MIPS {
                                    let (w, h) = self
                                        .blp
                                        .as_ref()
                                        .and_then(|b| b.mipmaps.get(i))
                                        .map(|m| (m.width, m.height))
                                        .unwrap_or((0, 0));

                                    mipmap_button_row(ui, &mut self.mip_visible[i], i, w, h);
                                }

                                // Кнопки All / None, поровну по ширине
                                let all_visible = self
                                    .mip_visible
                                    .iter()
                                    .all(|visible| *visible);
                                let any_visible = self
                                    .mip_visible
                                    .iter()
                                    .any(|visible| *visible);
                                ui.columns(2, |cols| {
                                    if cols[0]
                                        .add_enabled(!all_visible, Button::new(self.tr("mips-all")))
                                        .on_hover_text(self.tr("hint-mips-all"))
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.mip_visible.fill(true);
                                    }

                                    if cols[1]
                                        .add_enabled(any_visible, Button::new(self.tr("mips-none")))
                                        .on_hover_text(self.tr("hint-mips-none"))
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.mip_visible.fill(false);
                                    }
                                });
                            });
                        });

                        let _ = ui.allocate_exact_size(vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}

pub fn mipmap_button_row(ui: &mut Ui, on: &mut bool, i: usize, w: u32, h: u32) -> Response {
    let row_h = ui.spacing().interact_size.y;
    let width = ui.available_width();

    // Фиксированная ширина поля для w — чтобы "x" стоял на одном X у всех строк.
    const W_WIDTH: usize = 5; // под размеры до 99999; поменяй при необходимости

    // Текст: "#NN", пробел, w (леводополнённый), затем "x" и h.
    let text = format!("#{i:02} {w:>W_WIDTH$}x{h}", W_WIDTH = W_WIDTH);

    // Цвет текста: для off — disabled, для on — обычный.
    let v = &ui.style().visuals;
    let col = if *on {
        v.widgets.active.fg_stroke.color
    } else {
        // ослабляем до disabled
        v.widgets
            .inactive
            .fg_stroke
            .color
            .linear_multiply(v.disabled_alpha)
    };

    let label = RichText::new(text)
        .text_style(TextStyle::Monospace)
        .color(col);

    // Обычная кнопка, без fill — никаких фонов
    let resp = ui
        .add(Button::new(label).min_size(vec2(width, row_h)))
        .on_hover_cursor(CursorIcon::PointingHand);

    if resp.clicked() {
        *on = !*on;
    }
    resp
}
