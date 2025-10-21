#[cfg(debug_assertions)]
use eframe::egui::style::DebugOptions;
use eframe::egui::style::{HandleShape, Interaction, NumberFormatter, NumericColorSpace, ScrollAnimation, ScrollStyle, Selection, Style, TextCursorStyle, WidgetVisuals, Widgets};
use eframe::egui::{Color32, Context, CornerRadius, CursorIcon, FontId, Margin, Shadow, Spacing, Stroke, TextStyle, TextWrapMode, Visuals, vec2};
use eframe::emath::Rangef;
use eframe::epaint::AlphaFromCoverage;
use std::collections::BTreeMap;

pub fn apply_style(ctx: &Context) {
    // ── ПАЛИТРА (СВЕТЛЕЕ, ЧИСТЫЙ НЕОН) ─────────────────────────────────────────
    // Текст/белый
    let white = Color32::WHITE;
    let fg_main = Color32::from_rgb(236, 252, 255); // светлее, чуть голубее

    // Акценты
    let cyan = Color32::from_rgb(0, 235, 255); // ярче
    let cyan_hi = Color32::from_rgb(0, 255, 255); // максимум неона

    // Фоны (светлее всего набора)
    let bg_panel = Color32::from_rgb(18, 28, 36);
    let bg_window = Color32::from_rgba_unmultiplied(20, 34, 42, 236);
    let bg_weak = Color32::from_rgb(22, 36, 46);
    let bg_faint = Color32::from_rgb(24, 34, 40);
    let bg_extreme = Color32::from_rgb(12, 18, 22); // «очень тёмный», но светлее прежнего
    let bg_inactive = Color32::from_rgb(20, 38, 48);
    let bg_inactive_weak = Color32::from_rgb(26, 44, 54);
    let bg_hover = Color32::from_rgb(20, 56, 70);
    let bg_hover_weak = Color32::from_rgb(24, 64, 80);
    let bg_active = Color32::from_rgb(0, 170, 230);
    let bg_active_weak = Color32::from_rgb(0, 190, 250);
    let bg_open = Color32::from_rgb(26, 46, 58);
    let bg_open_weak = Color32::from_rgb(30, 54, 68);
    let code_bg = Color32::from_rgb(28, 42, 52);

    // Обводки/штрихи (чуть светлее/контрастнее)
    let stroke_lo = Color32::from_rgb(0, 120, 150);
    let stroke_md = Color32::from_rgb(0, 160, 190);
    let window_stroke = Color32::from_rgb(70, 120, 140);

    // Выделение/линки/статусы
    let selection_bg = Color32::from_rgba_premultiplied(0, 100, 130, 100);
    let selection_stroke = cyan_hi;
    let hyperlink_color = cyan;
    let warn_fg_color = Color32::from_rgb(255, 210, 0);
    let error_fg_color = Color32::from_rgb(255, 72, 112);

    // Тени
    let window_shadow_col = Color32::from_black_alpha(110);
    let popup_shadow_col = Color32::from_black_alpha(90);

    // Каретка ввода
    let text_cursor_col = cyan_hi;

    // ── ПОЛНАЯ ИНИЦИАЛИЗАЦИЯ СТИЛЯ (без default/clone) ─────────────────────────
    #[allow(deprecated)]
    ctx.set_style(Style {
        // 0) Глобальные оверрайды текста
        override_text_style: None,
        override_font_id: None,
        override_text_valign: None,

        // 1) Таблица стилей шрифтов
        text_styles: BTreeMap::from([
            (TextStyle::Small, FontId::proportional(11.0)), //
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Heading, FontId::proportional(18.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]),
        drag_value_text_style: TextStyle::Monospace,

        // 2) Формат чисел
        number_formatter: NumberFormatter::new(|value, decimals| {
            let max_dec = *decimals.end();
            format!("{:.*}", max_dec, value)
        }),

        // 3) Перенос текста
        wrap: None,                          // deprecated — не используем
        wrap_mode: Some(TextWrapMode::Wrap), // переносим по умолчанию

        // 4) Размеры/отступы/геометрия
        spacing: Spacing {
            item_spacing: vec2(8.0, 6.0),
            window_margin: Margin::symmetric(12, 10), // i8 в egui 0.32
            button_padding: vec2(12.0, 8.0),
            menu_margin: Margin::symmetric(10, 8),
            indent: 16.0,
            interact_size: vec2(36.0, 28.0),
            slider_width: 140.0,
            slider_rail_height: 2.0,
            combo_width: 160.0,
            text_edit_width: 280.0,
            icon_width: 18.0,
            icon_width_inner: 10.0,
            icon_spacing: 6.0,
            default_area_size: vec2(640.0, 480.0),
            tooltip_width: f32::INFINITY,
            menu_width: 360.0,
            menu_spacing: 8.0,
            indent_ends_with_horizontal_line: false,
            combo_height: 240.0,
            scroll: ScrollStyle::floating(),
        },

        // 5) Интеракция
        interaction: Interaction {
            interact_radius: 4.0, //
            resize_grab_radius_side: 6.0,
            resize_grab_radius_corner: 12.0,
            show_tooltips_only_when_still: true,
            tooltip_delay: 0.18,
            tooltip_grace_time: 0.08,
            selectable_labels: true,
            multi_widget_text_select: true,
        },

        // 6) Визуалы/цвета/состояния
        visuals: Visuals {
            dark_mode: true,
            text_alpha_from_coverage: AlphaFromCoverage::DARK_MODE_DEFAULT,

            override_text_color: Some(fg_main),
            weak_text_alpha: 0.66,
            weak_text_color: None,

            widgets: Widgets {
                noninteractive: WidgetVisuals {
                    bg_fill: bg_panel, //
                    weak_bg_fill: bg_weak,
                    bg_stroke: Stroke::new(1.0, stroke_lo),
                    fg_stroke: Stroke::new(1.0, fg_main),
                    corner_radius: CornerRadius::same(2u8),
                    expansion: 0.0,
                },
                inactive: WidgetVisuals {
                    bg_fill: bg_inactive,
                    weak_bg_fill: bg_inactive_weak,
                    bg_stroke: Stroke::new(0.0, fg_main), // плоско: без рамки у кнопок
                    fg_stroke: Stroke::new(1.0, fg_main),
                    corner_radius: CornerRadius::same(2u8),
                    expansion: 0.0,
                },
                hovered: WidgetVisuals {
                    bg_fill: bg_hover, //
                    weak_bg_fill: bg_hover_weak,
                    bg_stroke: Stroke::new(1.0, cyan),
                    fg_stroke: Stroke::new(1.0, white),
                    corner_radius: CornerRadius::same(2u8),
                    expansion: 0.0,
                },
                active: WidgetVisuals {
                    bg_fill: bg_active, //
                    weak_bg_fill: bg_active_weak,
                    bg_stroke: Stroke::new(1.0, cyan_hi),
                    fg_stroke: Stroke::new(1.0, white),
                    corner_radius: CornerRadius::same(2u8),
                    expansion: 0.0,
                },
                open: WidgetVisuals {
                    bg_fill: bg_open, //
                    weak_bg_fill: bg_open_weak,
                    bg_stroke: Stroke::new(1.0, stroke_md),
                    fg_stroke: Stroke::new(1.0, fg_main),
                    corner_radius: CornerRadius::same(2u8),
                    expansion: 0.0,
                },
            },

            selection: Selection {
                bg_fill: selection_bg, //
                stroke: Stroke::new(1.0, selection_stroke),
            },

            hyperlink_color,
            faint_bg_color: bg_faint,
            extreme_bg_color: bg_extreme,
            text_edit_bg_color: None,
            code_bg_color: code_bg,
            warn_fg_color,
            error_fg_color,

            window_corner_radius: CornerRadius::same(6u8),
            window_shadow: Shadow {
                offset: [10, 10], //
                blur: 15,
                spread: 0,
                color: window_shadow_col,
            },
            window_fill: bg_window,
            window_stroke: Stroke::new(1.0, window_stroke),
            window_highlight_topmost: true,

            menu_corner_radius: CornerRadius::same(4u8),
            panel_fill: bg_panel,
            popup_shadow: Shadow {
                offset: [6, 6], //
                blur: 8,
                spread: 0,
                color: popup_shadow_col,
            },

            resize_corner_size: 12.0,

            text_cursor: TextCursorStyle {
                stroke: Stroke::new(2.0, text_cursor_col), //
                preview: false,
                blink: true,
                on_duration: 0.6,
                off_duration: 0.4,
            },

            clip_rect_margin: 3.0,
            button_frame: true, // оставил как у тебя
            collapsing_header_frame: false,
            indent_has_left_vline: true,
            striped: false,
            slider_trailing_fill: false,
            handle_shape: HandleShape::Circle,
            interact_cursor: Some(CursorIcon::PointingHand),
            image_loading_spinners: true,
            numeric_color_space: NumericColorSpace::GammaByte,
            disabled_alpha: 0.4,
        },

        // 7) Время анимаций и отладка
        animation_time: 0.08,
        #[cfg(debug_assertions)]
        debug: DebugOptions {
            debug_on_hover: false, //
            debug_on_hover_with_all_modifiers: false,
            hover_shows_next: false,
            show_expand_width: false,
            show_expand_height: false,
            show_resize: false,
            show_interactive_widgets: false,
            show_widget_hits: false,
            show_unaligned: false,
        },

        // 8) Подсказки и ссылки
        explanation_tooltips: true,
        url_in_tooltip: true,

        // 9) Скролл-анимации и режимы
        always_scroll_the_only_direction: true,
        scroll_animation: ScrollAnimation::new(1100.0, Rangef::new(0.08, 0.35)),
        compact_menu_style: true,
    });
}
