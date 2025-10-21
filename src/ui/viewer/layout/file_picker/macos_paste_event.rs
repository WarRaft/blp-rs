#![cfg(target_os = "macos")]
#![allow(unexpected_cfgs, deprecated)] // оставляю как просил

use cocoa::appkit::NSPasteboard;
use cocoa::appkit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSInteger, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::CStr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static CMDV_EVENT: AtomicBool = AtomicBool::new(false);

// ==================== SAFE WRAPPERS ====================
// базовые

#[inline]
fn ns_app() -> id {
    unsafe { NSApplication::sharedApplication(nil) }
}

#[inline]
fn ns_menu_new() -> id {
    unsafe { NSMenu::new(nil) }
}

#[inline]
fn ns_menu_item_new() -> id {
    unsafe { NSMenuItem::new(nil) }
}

#[inline]
fn nsstring(s: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(s) }
}

// menu API

#[inline]
fn menu_number_of_items(menu: id) -> NSInteger {
    unsafe { msg_send![menu, numberOfItems] }
}

#[inline]
fn menu_item_at(menu: id, idx: NSInteger) -> id {
    unsafe { msg_send![menu, itemAtIndex: idx] }
}

#[inline]
fn menu_add_item(menu: id, item: id) {
    unsafe {
        let _: () = msg_send![menu, addItem: item];
    }
}

// menu item API

#[inline]
fn menu_item_submenu(item: id) -> id {
    unsafe { msg_send![item, submenu] }
}

#[inline]
fn menu_item_set_title(item: id, title: &str) {
    unsafe {
        let _: () = msg_send![item, setTitle: nsstring(title)];
    }
}

#[inline]
fn menu_item_set_submenu(item: id, submenu: id) {
    unsafe {
        let _: () = msg_send![item, setSubmenu: submenu];
    }
}

#[inline]
fn menu_item_key_equivalent(item: id) -> String {
    unsafe {
        let key: id = msg_send![item, keyEquivalent];
        let c: *const std::os::raw::c_char = msg_send![key, UTF8String];
        if c.is_null() {
            String::new()
        } else {
            CStr::from_ptr(c)
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[inline]
fn menu_item_modifier_mask(item: id) -> NSEventModifierFlags {
    unsafe { msg_send![item, keyEquivalentModifierMask] }
}

#[inline]
fn menu_item_set_action(item: id, action: Sel) {
    unsafe {
        let _: () = msg_send![item, setAction: action];
    }
}

#[inline]
fn menu_item_set_target(item: id, target: id) {
    unsafe {
        let _: () = msg_send![item, setTarget: target];
    }
}

#[inline]
fn menu_item_with_title_action_key(title: &str, action: Sel, key: &str) -> id {
    unsafe { NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(nsstring(title), action, nsstring(key)) }
}

#[inline]
fn menu_item_set_key_mask(item: id, mask: NSEventModifierFlags) {
    unsafe {
        let _: () = msg_send![item, setKeyEquivalentModifierMask: mask];
    }
}

// app actions

#[inline]
fn app_send_paste() {
    unsafe {
        let _: bool = msg_send![ns_app(), sendAction: sel!(paste:) to: nil from: nil];
    }
}

// class helpers

#[inline]
fn class_get_or_register() -> Option<&'static Class> {
    unsafe {
        if let Some(cls) = Class::get("RustPasteTarget") {
            return Some(cls);
        }
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("RustPasteTarget", superclass)?;
        decl.add_method(sel!(pasteCmdV:), paste_cmdv as extern "C" fn(&Object, Sel, id));
        decl.add_method(sel!(validateMenuItem:), validate_menu_item as extern "C" fn(&Object, Sel, id) -> bool);
        Some(decl.register())
    }
}

#[inline]
fn app_main_menu() -> id {
    unsafe { msg_send![ns_app(), mainMenu] }
}

/// Возвращает первый file:// из буфера (Finder → Cmd+C на файле)
#[inline]
pub fn pasteboard_file_path() -> Option<PathBuf> {
    unsafe {
        let pb: id = NSPasteboard::generalPasteboard(nil);
        if pb == nil {
            return None;
        }

        // 1) Быстро: [NSURL URLFromPasteboard:pb]
        let url: id = msg_send![class!(NSURL), URLFromPasteboard: pb];
        if url != nil {
            let is_file: bool = msg_send![url, isFileURL];
            if is_file {
                let ns_path: id = msg_send![url, path];
                if ns_path != nil {
                    let c: *const std::os::raw::c_char = msg_send![ns_path, UTF8String];
                    if !c.is_null() {
                        let s = CStr::from_ptr(c)
                            .to_string_lossy()
                            .into_owned();
                        return Some(PathBuf::from(s));
                    }
                }
            }
        }

        // 2) Запасной путь: readObjectsForClasses:@[NSURL] options:nil
        let nsurl_class: id = msg_send![class!(NSURL), class];
        let classes: id = msg_send![class!(NSArray), arrayWithObject: nsurl_class];
        let urls: id = msg_send![pb, readObjectsForClasses: classes options: nil];
        if urls != nil {
            let count: isize = msg_send![urls, count];
            if count > 0 {
                let url: id = msg_send![urls, objectAtIndex: 0];
                let is_file: bool = msg_send![url, isFileURL];
                if is_file {
                    let ns_path: id = msg_send![url, path];
                    if ns_path != nil {
                        let c: *const std::os::raw::c_char = msg_send![ns_path, UTF8String];
                        if !c.is_null() {
                            let s = CStr::from_ptr(c)
                                .to_string_lossy()
                                .into_owned();
                            return Some(PathBuf::from(s));
                        }
                    }
                }
            }
        }

        // 3) Легаси: NSFilenamesPboardType → NSArray<NSString*>
        let ty = nsstring("NSFilenamesPboardType");
        let arr: id = msg_send![pb, propertyListForType: ty];
        if arr != nil {
            let count: isize = msg_send![arr, count];
            if count > 0 {
                let s: id = msg_send![arr, objectAtIndex: 0];
                if s != nil {
                    let c: *const std::os::raw::c_char = msg_send![s, UTF8String];
                    if !c.is_null() {
                        let s = CStr::from_ptr(c)
                            .to_string_lossy()
                            .into_owned();
                        return Some(PathBuf::from(s));
                    }
                }
            }
        }

        None
    }
}

// ==================== HANDLERS ====================

// ---- Обработчики пункта меню (extern "C") ----
extern "C" fn paste_cmdv(_this: &Object, _cmd: Sel, _sender: id) {
    // 1) сигналим в UI
    CMDV_EVENT.store(true, Ordering::SeqCst);
    // 2) пробрасываем системное paste:, чтобы текстовые поля жили
    app_send_paste();
}

// Можно всегда true, или сделать умнее через validateMenuItem
extern "C" fn validate_menu_item(_this: &Object, _cmd: Sel, _item: id) -> bool {
    true
}

fn ensure_target_class() -> Option<&'static Class> {
    class_get_or_register()
}

/// Ищем существующий `Paste (⌘V)` и перепривязываем action/target.
/// Если его нет — аккуратно ДОБАВЛЯЕМ только один пункт `Edit→Paste` (без setMainMenu_!).
unsafe fn hook_or_create_paste_item() -> bool {
    let main: id = app_main_menu();
    if main == nil {
        return false;
    }

    let top_n = menu_number_of_items(main);

    // 1) Пытаемся НАЙТИ существующий Paste
    for i in 0..top_n {
        let top_item = menu_item_at(main, i);
        if top_item == nil {
            continue;
        }
        let submenu = menu_item_submenu(top_item);
        if submenu == nil {
            continue;
        }
        let sub_n = menu_number_of_items(submenu);
        for j in 0..sub_n {
            let it = menu_item_at(submenu, j);
            if it == nil {
                continue;
            }
            let eq = menu_item_key_equivalent(it);
            let mask = menu_item_modifier_mask(it);
            if eq == "v" && mask.contains(NSEventModifierFlags::NSCommandKeyMask) {
                let Some(cls) = ensure_target_class() else {
                    return false;
                };
                let target: id = msg_send![cls, new];
                menu_item_set_action(it, sel!(pasteCmdV:));
                menu_item_set_target(it, target);
                return true;
            }
        }
    }

    // 2) Если не нашли — аккуратно ДОБАВИМ «Edit → Paste» (не трогаем main menu целиком)
    // Ищем подменю, похожее на Edit (по наличию Cut/Copy).
    let mut edit_sub: id = nil;
    'outer: for i in 0..top_n {
        let ti = menu_item_at(main, i);
        if ti == nil {
            continue;
        }
        let sm = menu_item_submenu(ti);
        if sm == nil {
            continue;
        }
        let sub_n = menu_number_of_items(sm);
        let mut has_cut = false;
        let mut has_copy = false;
        for j in 0..sub_n {
            let it = menu_item_at(sm, j);
            if it == nil {
                continue;
            }
            let eq = menu_item_key_equivalent(it);
            let mask = menu_item_modifier_mask(it);
            if eq == "x" && mask.contains(NSEventModifierFlags::NSCommandKeyMask) {
                has_cut = true;
            }
            if eq == "c" && mask.contains(NSEventModifierFlags::NSCommandKeyMask) {
                has_copy = true;
            }
            if has_cut && has_copy {
                edit_sub = sm;
                break 'outer;
            }
        }
    }

    // Если Edit не нашли — создадим top-level «Edit» с подменю (без setMainMenu_)
    if edit_sub == nil {
        let edit_item = ns_menu_item_new();
        menu_item_set_title(edit_item, "Edit");
        menu_add_item(main, edit_item);
        let menu = ns_menu_new();
        menu_item_set_submenu(edit_item, menu);
        edit_sub = menu;
    }

    // Добавляем сам пункт Paste (Cmd+V) с нашим обработчиком
    let paste = menu_item_with_title_action_key("Paste", sel!(pasteCmdV:), "v");
    menu_item_set_key_mask(paste, NSEventModifierFlags::NSCommandKeyMask);

    let Some(cls) = ensure_target_class() else {
        return false;
    };
    let target: id = msg_send![cls, new];
    menu_item_set_target(paste, target);
    menu_add_item(edit_sub, paste);

    true
}

/// Дёргай каждый кадр, пока не зацепимся за Cmd+V. Потом отключится сам.
pub fn tick_ensure_cmdv_event() {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.load(Ordering::Relaxed) {
        return;
    }
    unsafe {
        if hook_or_create_paste_item() {
            DONE.store(true, Ordering::Relaxed);
        }
    }
}

/// Забрать одноразовый флаг события Cmd+V (true если было в этом кадре)
pub fn take_cmdv_event() -> bool {
    CMDV_EVENT.swap(false, Ordering::SeqCst)
}
