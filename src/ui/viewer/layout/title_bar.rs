use crate::ui::viewer::app::App;
use eframe::egui::{Align, Color32, Context, CursorIcon, FontId, Frame, Layout, Pos2, Response, Sense, Shape, Stroke, TopBottomPanel, Ui, Vec2, ViewportCommand};

impl App {
    pub(crate) fn draw_title_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("custom_title_bar")
            .show_separator_line(false)
            .exact_height(30.0)
            .frame(Frame { fill: Color32::from_rgba_unmultiplied(8, 32, 44, 200), ..Default::default() })
            .show(ctx, |ui| {
                let title_bar_rect = ui.max_rect();

                // --- ЛЕВЫЙ БЕЙДЖ "blp" со скошенным правым краем ---
                let label_text = "blp";
                let font = FontId::proportional(16.0);
                let galley = ui.fonts_mut(|f| f.layout_no_wrap(label_text.to_string(), font.clone(), Color32::WHITE));

                // позиция текста: слева с отступом, по центру по вертикали
                let pad_x = 10.0;
                let pad_y = 3.0;
                let text_pos = Pos2::new(
                    title_bar_rect.left() + pad_x, // немного отступаем от края панели
                    title_bar_rect.center().y - galley.size().y * 0.5,
                );

                // фон-бейдж: полигон с диагональным правым краем
                let x0 = text_pos.x - pad_x;
                let y0 = text_pos.y - pad_y;
                let x1 = text_pos.x + galley.size().x + pad_x;
                let y1 = text_pos.y + galley.size().y + pad_y;
                let slope = 12.0; // «скос» правого края

                let pts = vec![
                    Pos2::new(x0, y0),              // левый верх
                    Pos2::new(x1 - slope, y0),      // правый верх со сдвигом
                    Pos2::new(x1, (y0 + y1) * 0.5), // «носик» по центру справа
                    Pos2::new(x1 - slope, y1),      // правый низ со сдвигом
                    Pos2::new(x0, y1),              // левый низ
                ];
                ui.painter().add(Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(10, 180, 250, 80), // насыщённее, чем фон панели
                    Stroke::NONE,
                ));
                ui.painter()
                    .galley(text_pos, galley, Color32::WHITE);

                // --- СЛЕДОМ СПРАВА: отдельный бейдж версии ---
                let ver_text = env!("CARGO_PKG_VERSION");
                let ver_galley = ui.fonts_mut(|f| f.layout_no_wrap(ver_text.to_string(), font.clone(), Color32::WHITE));

                let ver_gap = 6.0; // зазор между бейджами
                let ver_text_pos = Pos2::new(
                    x1 + ver_gap, // начинаем после "blp"
                    title_bar_rect.center().y - ver_galley.size().y * 0.5,
                );

                ui.painter()
                    .galley(ver_text_pos, ver_galley, Color32::WHITE);

                // --- СПРАВА: red → green → yellow ---
                let (close_resp, zoom_resp, min_resp) = ui
                    .with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(6.0);
                        let close_resp = macos_dot(ui, TrafficKind::Close).on_hover_cursor(CursorIcon::PointingHand);
                        ui.add_space(6.0);
                        let zoom_resp = macos_dot_zoom(ui, self.maximized).on_hover_cursor(CursorIcon::PointingHand);
                        ui.add_space(6.0);
                        let min_resp = macos_dot(ui, TrafficKind::Minimize).on_hover_cursor(CursorIcon::PointingHand);
                        (close_resp, zoom_resp, min_resp)
                    })
                    .inner;

                // действия
                if min_resp.clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                if zoom_resp.clicked() {
                    self.maximized = !self.maximized;
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(self.maximized));
                }
                if close_resp.clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }

                // курсор "Grab/Move" над drag-зоной (всё, кроме кружков)
                if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                    let over_btns = min_resp.rect.contains(p) || zoom_resp.rect.contains(p) || close_resp.rect.contains(p);
                    if title_bar_rect.contains(p) && !over_btns {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
                    }
                }

                // drag-area — всё кроме кружков
                let pointer = ui.input(|i| i.pointer.clone());
                if pointer.primary_down() {
                    if let Some(pos) = pointer.interact_pos() {
                        let over = min_resp.rect.contains(pos) || zoom_resp.rect.contains(pos) || close_resp.rect.contains(pos);
                        if title_bar_rect.contains(pos) && !over {
                            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                        }
                    }
                }
            });
    }
}

#[derive(Copy, Clone)]
enum TrafficKind {
    Close,
    Minimize,
}

const MACOS_DOT_SIZE: f32 = 18.0;

fn macos_dot(ui: &mut Ui, kind: TrafficKind) -> Response {
    let size = Vec2::splat(MACOS_DOT_SIZE);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let center = rect.center();

    let (base, hover_stroke) = match kind {
        TrafficKind::Close => (Color32::from_rgb(255, 95, 86), Color32::from_rgba_unmultiplied(0, 0, 0, 100)),     // red
        TrafficKind::Minimize => (Color32::from_rgb(255, 189, 46), Color32::from_rgba_unmultiplied(0, 0, 0, 100)), // yellow
    };

    ui.painter()
        .circle_filled(center, MACOS_DOT_SIZE * 0.5, base);

    if resp.hovered() {
        ui.painter()
            .circle_stroke(center, MACOS_DOT_SIZE * 0.5, Stroke { width: 1.0, color: hover_stroke });

        match kind {
            TrafficKind::Close => {
                let r = MACOS_DOT_SIZE * 0.28;
                ui.painter()
                    .line_segment([Pos2::new(center.x - r, center.y - r), Pos2::new(center.x + r, center.y + r)], Stroke { width: 1.5, color: Color32::BLACK });
                ui.painter()
                    .line_segment([Pos2::new(center.x - r, center.y + r), Pos2::new(center.x + r, center.y - r)], Stroke { width: 1.5, color: Color32::BLACK });
            }
            TrafficKind::Minimize => {
                let r = MACOS_DOT_SIZE * 0.30;
                ui.painter()
                    .line_segment([Pos2::new(center.x - r, center.y), Pos2::new(center.x + r, center.y)], Stroke { width: 2.0, color: Color32::BLACK });
            }
        }
    }

    resp
}

fn macos_dot_zoom(ui: &mut Ui, inward: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(MACOS_DOT_SIZE), Sense::click());
    let c = rect.center();
    let r = MACOS_DOT_SIZE * 0.5;

    // круг
    ui.painter()
        .circle_filled(c, r, Color32::from_rgb(39, 201, 63));

    // показываем знак на hover (убери if, если нужно всегда видно)
    if resp.hovered() {
        ui.painter()
            .circle_stroke(c, r, Stroke { width: 1.0, color: Color32::from_rgba_unmultiplied(0, 0, 0, 100) });

        // диагональ ↘ (единичный вектор) и её перпендикуляр
        let u = Vec2::new(1.0, 1.0).normalized();
        let n = Vec2::new(-u.y, u.x);

        // параметры глифа
        let tip_off = if inward { 1. } else { r * 0.8 }; // смещение носика от центра вдоль диагонали
        let height = r * 0.6; // "длина" треугольника
        let base_w = r * 0.9; // ширина основания

        // рисовалка "по носику"
        let tri_tip = |tip: Pos2, dir: Vec2| {
            let d = dir.normalized();
            let base_c = tip - d * height;
            let a = base_c + n * (base_w * 0.5);
            let b = base_c - n * (base_w * 0.5);
            ui.painter()
                .add(Shape::convex_polygon(vec![tip, a, b], Color32::BLACK, Stroke::NONE));
        };

        // положения носиков на диагонали
        let tip_tl = c - u * tip_off; // верх-лево
        let tip_br = c + u * tip_off; // низ-право

        if inward {
            // внутрь: ↘ у верх-левого, ↖ у нижне-правого
            tri_tip(tip_tl, u); // к центру
            tri_tip(tip_br, -u); // к центру
        } else {
            // наружу: ↖ у верх-левого, ↘ у нижне-правого
            tri_tip(tip_tl, -u); // наружу
            tri_tip(tip_br, u); // наружу
        }
    }

    resp
}
