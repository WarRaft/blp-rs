use eframe::egui::{Context, Event, Key, KeyboardShortcut, Modifiers};

pub(in crate::ui::viewer) fn hotkey_pressed(ctx: &Context, key: Key) -> bool {
    // 1) Нормальный путь: шорткат (Cmd на mac / Ctrl на win/linux)
    let via_shortcut = ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, key)));
    if via_shortcut {
        return true;
    }

    // 2) Фолбэк: комбо (съедаем точную пару command+key, Shift допустим, Alt — нет)
    let via_combo = ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, key));
    if via_combo {
        return true;
    }

    // 3) Сырой ивент (на случай экзотики на macOS)
    ctx.input(|i| {
        i.raw.events.iter().any(|e| {
            matches!(
                e,
                Event::Key {
                    key: k,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if *k == key && modifiers.command && !modifiers.alt
            )
        })
    })
}
