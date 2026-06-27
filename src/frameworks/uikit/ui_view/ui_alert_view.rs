/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertView` — shows an SDL2 message box dialog.

use crate::frameworks::core_graphics::cg_affine_transform::CGAffineTransform;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::{ns_string, NSInteger, NSUInteger};
use crate::objc::{
    id, msg, msg_class, msg_super, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::window;

pub type UIAlertViewStyle = NSInteger;
pub const UIAlertViewStyleDefault: UIAlertViewStyle = 0;
pub const UIAlertViewStyleSecureTextInput: UIAlertViewStyle = 1;
pub const UIAlertViewStylePlainTextInput: UIAlertViewStyle = 2;
pub const UIAlertViewStyleLoginAndPasswordInput: UIAlertViewStyle = 3;

#[derive(Default)]
pub struct UIAlertViewHostObject {
    title: id,
    message: id,
    delegate: id,
    button_titles: id,
    cancel_button_index: NSInteger,
    visible: bool,
    alert_view_style: UIAlertViewStyle,
    tag: NSInteger,
}
impl HostObject for UIAlertViewHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIAlertView: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIAlertViewHostObject {
        title:               nil,
        message:             nil,
        delegate:            nil,
        button_titles:       nil,
        cancel_button_index: -1,
        visible:             false,
        alert_view_style:    UIAlertViewStyleDefault,
        tag:                 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTitle:(id)title
            message:(id)message
           delegate:(id)delegate
  cancelButtonTitle:(id)cancel_title
  otherButtonTitles:(id)other_titles {

    // ВАЖНО: Вызов базового инициализатора для корректной регистрации объекта
    let this: id = msg_super![env; this init];

    let buttons: id = msg_class![env; NSMutableArray new];
    retain(env, title);
    retain(env, message);
    {
        // ИСПРАВЛЕНИЕ: Добавлен `mut`, так как изменение полей `RefMut` требует
        // мутабельности переменной
        let host = env.objc.borrow_mut::<UIAlertViewHostObject>(this);
        host.title    = title;
        host.message  = message;
        host.delegate = delegate;
        host.button_titles = buttons;
    }
    if cancel_title != nil {
        let idx: NSUInteger = msg![env; buttons count];
        let _: () = msg![env; buttons addObject:cancel_title];
        env.objc.borrow_mut::<UIAlertViewHostObject>(this).cancel_button_index = idx as NSInteger;
    }
    if other_titles != nil {
        let _: () = msg![env; buttons addObject:other_titles];
    }
    let title_str = if title != nil { ns_string::to_rust_string(env, title).into_owned() } else { "(nil)".into() };
    let msg_str   = if message != nil { ns_string::to_rust_string(env, message).into_owned() } else { "(nil)".into() };
    log!("UIAlertView init title={:?} message={:?}", title_str, msg_str);
    this
}

- (())dealloc {
    // ИСПРАВЛЕНИЕ: Блокируем `host` в узком scope, чтобы снять заимствование до
    // `release`
    let (title, message, buttons) = {
        let host = env.objc.borrow::<UIAlertViewHostObject>(this);
        (host.title, host.message, host.button_titles)
    };
    release(env, title);
    release(env, message);
    release(env, buttons);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)title   { env.objc.borrow::<UIAlertViewHostObject>(this).title }
- (id)message { env.objc.borrow::<UIAlertViewHostObject>(this).message }
- (id)delegate { env.objc.borrow::<UIAlertViewHostObject>(this).delegate }
- (())setTitle:(id)title {
    let old = env.objc.borrow::<UIAlertViewHostObject>(this).title;
    release(env, old); retain(env, title);
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).title = title;
}
- (())setMessage:(id)message {
    let old = env.objc.borrow::<UIAlertViewHostObject>(this).message;
    release(env, old); retain(env, message);
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).message = message;
}
- (())setDelegate:(id)delegate {
    // Делегаты в UIKit не удерживаются!
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).delegate = delegate;
}
- (NSInteger)tag { env.objc.borrow::<UIAlertViewHostObject>(this).tag }
- (())setTag:(NSInteger)tag { env.objc.borrow_mut::<UIAlertViewHostObject>(this).tag = tag; }
- (UIAlertViewStyle)alertViewStyle { env.objc.borrow::<UIAlertViewHostObject>(this).alert_view_style }
- (())setAlertViewStyle:(UIAlertViewStyle)style { env.objc.borrow_mut::<UIAlertViewHostObject>(this).alert_view_style = style; }
- (bool)isVisible { env.objc.borrow::<UIAlertViewHostObject>(this).visible }

- (NSInteger)addButtonWithTitle:(id)title {
    let buttons = env.objc.borrow::<UIAlertViewHostObject>(this).button_titles;
    let idx: NSUInteger = msg![env; buttons count];
    let _: () = msg![env; buttons addObject:title];
    idx as NSInteger
}
- (NSUInteger)numberOfButtons {
    let buttons = env.objc.borrow::<UIAlertViewHostObject>(this).button_titles;
    msg![env; buttons count]
}
- (id)buttonTitleAtIndex:(NSInteger)index {
    let buttons = env.objc.borrow::<UIAlertViewHostObject>(this).button_titles;
    let count: NSUInteger = msg![env; buttons count];
    if index < 0 || index as NSUInteger >= count { return nil; }
    msg![env; buttons objectAtIndex:(index as NSUInteger)]
}
- (NSInteger)cancelButtonIndex { env.objc.borrow::<UIAlertViewHostObject>(this).cancel_button_index }
- (())setCancelButtonIndex:(NSInteger)index { env.objc.borrow_mut::<UIAlertViewHostObject>(this).cancel_button_index = index; }
- (NSInteger)firstOtherButtonIndex {
    // ИСПРАВЛЕНИЕ: Скоуп не даст возникнуть ошибке заимствования на `msg![env;
    // ...]`
    let (buttons, cancel) = {
        let host = env.objc.borrow::<UIAlertViewHostObject>(this);
        (host.button_titles, host.cancel_button_index)
    };
    let count: NSUInteger = msg![env; buttons count];
    for i in 0..count { if i as NSInteger != cancel { return i as NSInteger; } }
    -1
}
- (id)textFieldAtIndex:(NSInteger)_index { nil }

- (())addSubview:(id)_view {
    // UIAlertView doesn't support subviews in touchHLE (SDL2 dialog
    // implementation)
    log_dbg!("UIAlertView addSubview: ignored");
}
- (())removeFromSuperview {
    log_dbg!("UIAlertView removeFromSuperview: ignored");
}
- (())setHidden:(bool)_hidden {
    log_dbg!("UIAlertView setHidden: ignored");
}
- (CGRect)frame {
    CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: 0.0, height: 0.0 } }
}
- (())setFrame:(CGRect)_frame {
    log_dbg!("UIAlertView setFrame: ignored");
}

// UIAlertView inherits from UIView in real iOS.  We can't structurally
// derive from UIView here (SDL2 dialog), so we stub the selectors that
// guest apps actually invoke on alert views.
- (())setTransform:(CGAffineTransform)_transform {
    // UIAlertView in touchHLE is rendered via SDL2 system dialog,
    // so geometric transforms are not applicable.
    log_dbg!("UIAlertView setTransform: ignored (SDL2 dialog)");
}

- (id)viewWithTag:(NSInteger)tag {
    // Real UIAlertView would search subviews, but touchHLE doesn't manage
    // a subview hierarchy for alerts.  Return self if the tag matches,
    // otherwise nil (Apple semantics: receiver is searched first).
    let own_tag: NSInteger = msg![env; this tag];
    if own_tag == tag { return this; }
    nil
}

- (CGSize)sizeThatFits:(CGSize)size {
    // В iOS этот метод возвращает оптимальный размер на основе содержимого.
    // Так как touchHLE выводит SDL2-диалог, размер контролируется самой ОС,
    // поэтому мы пробрасываем текущий запрошенный размер дальше.
    size
}

- (())sizeToFit {
    // 1. Получаем текущий frame
    let frame: CGRect = msg![env; this frame];

    // 2. ВАЖНО: Выносим размер в отдельную переменную.
    // Макрос msg! не принимает "frame.size" как аргумент после двоеточия.
    let current_size = frame.size;

    // 3. Запрашиваем подходящий размер
    let new_size: CGSize = msg![env; this sizeThatFits:current_size];

    // 4. Формируем новый frame и применяем его
    let new_frame = CGRect { origin: frame.origin, size: new_size };
    let _: () = msg![env; this setFrame:new_frame];
}

- (())show {
    log!("UIAlertView show (SDL2 dialog)");
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).visible = true;

    let (title, message, buttons, cancel_index) = {
        let h = env.objc.borrow::<UIAlertViewHostObject>(this);
        (h.title, h.message, h.button_titles, h.cancel_button_index)
    };

    // Raw (un-substituted) strings, used to decide whether the alert
    // actually has anything to show the user.
    let raw_title: String = if title != nil {
        ns_string::to_rust_string(env, title).into_owned()
    } else { String::new() };
    let raw_message: String = if message != nil {
        ns_string::to_rust_string(env, message).into_owned()
    } else { String::new() };

    // touchHLE renders UIAlertView as a *blocking* SDL2 system dialog,
    // whereas real iOS `-[UIAlertView show]` is asynchronous and returns
    // immediately. Some apps (notably Outfit7 titles like Talking Angela)
    // create a content-less alert — empty/`nil` title *and* message — as a
    // transient placeholder that they dismiss programmatically once some
    // background work finishes. Presenting a blocking modal for such an
    // alert freezes the app behind an empty dialog box that the user can
    // never meaningfully act on. Since there is nothing to display, skip
    // the native dialog and simulate an immediate dismissal so the guest's
    // run loop keeps going (matching iOS's non-blocking semantics). Any
    // delegate callbacks are still delivered via dismissWithClickedButtonIndex.
    if raw_title.trim().is_empty() && raw_message.trim().is_empty() {
        log!(
            "UIAlertView show: empty title and message; \
             skipping blocking SDL2 dialog and dismissing asynchronously"
        );
        let dismiss_index = if cancel_index >= 0 { cancel_index } else { 0 };
        let _: () = msg![env; this dismissWithClickedButtonIndex:dismiss_index animated:false];
        return;
    }

    let title_str: String = if raw_title.is_empty() { "Alert".into() } else { raw_title };
    let message_str: String = raw_message;

    let btn_count: NSUInteger = msg![env; buttons count];
    let mut btn_strings: Vec<String> = Vec::new();
    for i in 0..btn_count {
        let btn: id = msg![env; buttons objectAtIndex:i];
        btn_strings.push(if btn != nil {
            ns_string::to_rust_string(env, btn).into_owned()
        } else { format!("Button {}", i) });
    }
    if btn_strings.is_empty() { btn_strings.push("OK".into()); }

    let btn_refs: Vec<&str> = btn_strings.iter().map(|s| s.as_str()).collect();
    let clicked = window::show_alert_dialog(env, &title_str, &message_str, &btn_refs);

    let dismiss_index = if clicked >= 0 && (clicked as NSUInteger) < btn_count {
        clicked as NSInteger
    } else if cancel_index >= 0 {
        cancel_index
    } else { 0 };

    let _: () = msg![env; this dismissWithClickedButtonIndex:dismiss_index animated:false];
}

- (())dismissWithClickedButtonIndex:(NSInteger)button_index animated:(bool)_animated {
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).visible = false;
    let delegate = env.objc.borrow::<UIAlertViewHostObject>(this).delegate;

    // Честно проверяем, не был ли делегат удален (isa != 0)
    if delegate != nil {
        let isa: u32 = env.mem.read(delegate.cast());
        if isa != 0 {
            if let Some(sel) = env.objc.lookup_selector("alertView:clickedButtonAtIndex:") {
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds { let _: () = msg![env; delegate alertView:this clickedButtonAtIndex:button_index]; }
            }
            if let Some(sel) = env.objc.lookup_selector("alertView:willDismissWithButtonIndex:") {
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds { let _: () = msg![env; delegate alertView:this willDismissWithButtonIndex:button_index]; }
            }
            if let Some(sel) = env.objc.lookup_selector("alertView:didDismissWithButtonIndex:") {
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds { let _: () = msg![env; delegate alertView:this didDismissWithButtonIndex:button_index]; }
            }
        }
    }
}

- (id)description {
    let (title, visible) = { let h = env.objc.borrow::<UIAlertViewHostObject>(this); (h.title, h.visible) };
    let title_str = if title != nil { ns_string::to_rust_string(env, title).into_owned() } else { "(nil)".into() };
    let s = format!("<UIAlertView: {:?}; title={:?}; visible={}>", this, title_str, visible);
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};
