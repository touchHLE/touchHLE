/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertController` and `UIAlertAction`.
//!
//! Apple documentation:
//! * https://developer.apple.com/documentation/uikit/uialertcontroller
//! * https://developer.apple.com/documentation/uikit/uialertaction

use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::{ns_string, NSInteger, NSUInteger};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, nil, objc_classes,
    objc_retainBlock, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::window;

pub type UIAlertActionStyle = NSInteger;
pub const UIAlertActionStyleDefault: UIAlertActionStyle = 0;
pub const UIAlertActionStyleCancel: UIAlertActionStyle = 1;
pub const UIAlertActionStyleDestructive: UIAlertActionStyle = 2;

pub type UIAlertControllerStyle = NSInteger;
pub const UIAlertControllerStyleActionSheet: UIAlertControllerStyle = 0;
pub const UIAlertControllerStyleAlert: UIAlertControllerStyle = 1;

#[derive(Default)]
struct UIAlertActionHostObject {
    title: id,
    style: UIAlertActionStyle,
    handler: id,
    enabled: bool,
}
impl HostObject for UIAlertActionHostObject {}

#[derive(Default)]
struct UIAlertControllerHostObject {
    superclass: super::ui_view_controller::UIViewControllerHostObject,
    title: id,
    message: id,
    preferred_style: UIAlertControllerStyle,
    actions: id,
    text_fields: id,
    preferred_action: id,
    presented: bool,
}
impl_HostObject_with_superclass!(UIAlertControllerHostObject);

fn invoke_block(env: &mut crate::Environment, block: id, object: id) {
    if block == nil {
        return;
    }
    let invoke_ptr: u32 = env.mem.read(block.cast::<u32>() + 3u32);
    if invoke_ptr == 0 {
        log!("Warning: block {:?} has NULL invoke pointer; skipping.", block);
        return;
    }
    use crate::abi::CallFromHost;
    let invoke = crate::abi::GuestFunction::from_addr_with_thumb_bit(invoke_ptr);
    let block_arg: crate::mem::ConstVoidPtr =
        crate::mem::Ptr::from_bits(block.to_bits()).cast_const();
    let _: () = invoke.call_from_host(env, (block_arg, object));
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIAlertAction: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIAlertActionHostObject {
        title: nil,
        style: UIAlertActionStyleDefault,
        handler: nil,
        enabled: true,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)actionWithTitle:(id)title style:(UIAlertActionStyle)style handler:(id)handler {
    let action: id = msg_class![env; UIAlertAction alloc];
    let action: id = msg![env; action initWithTitle:title style:style handler:handler];
    crate::objc::autorelease(env, action)
}

- (id)initWithTitle:(id)title style:(UIAlertActionStyle)style handler:(id)handler {
    let title = retain(env, title);
    let handler = objc_retainBlock(env, handler);
    let host = env.objc.borrow_mut::<UIAlertActionHostObject>(this);
    host.title = title;
    host.style = style;
    host.handler = handler;
    host.enabled = true;
    this
}

- (())dealloc {
    let UIAlertActionHostObject { title, handler, .. } =
        std::mem::take(env.objc.borrow_mut::<UIAlertActionHostObject>(this));
    release(env, title);
    release(env, handler);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)title {
    env.objc.borrow::<UIAlertActionHostObject>(this).title
}

- (UIAlertActionStyle)style {
    env.objc.borrow::<UIAlertActionHostObject>(this).style
}

- (bool)isEnabled {
    env.objc.borrow::<UIAlertActionHostObject>(this).enabled
}

- (())setEnabled:(bool)enabled {
    env.objc.borrow_mut::<UIAlertActionHostObject>(this).enabled = enabled;
}

- (id)handler {
    env.objc.borrow::<UIAlertActionHostObject>(this).handler
}

@end

@implementation UIAlertController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIAlertControllerHostObject {
        superclass: Default::default(),
        title: nil,
        message: nil,
        preferred_style: UIAlertControllerStyleAlert,
        actions: nil,
        text_fields: nil,
        preferred_action: nil,
        presented: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)alertControllerWithTitle:(id)title message:(id)message preferredStyle:(UIAlertControllerStyle)style {
    let controller: id = msg_class![env; UIAlertController alloc];
    let controller: id = msg![env; controller initWithTitle:title message:message preferredStyle:style];
    crate::objc::autorelease(env, controller)
}

- (id)initWithTitle:(id)title message:(id)message preferredStyle:(UIAlertControllerStyle)style {
    let _: id = msg![env; this init];
    let title = retain(env, title);
    let message = retain(env, message);
    let actions: id = msg_class![env; NSMutableArray new];
    let host = env.objc.borrow_mut::<UIAlertControllerHostObject>(this);
    host.title = title;
    host.message = message;
    host.preferred_style = style;
    host.actions = actions;
    this
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<UIAlertControllerHostObject>(this));
    release(env, host.title);
    release(env, host.message);
    release(env, host.actions);
    release(env, host.text_fields);
    release(env, host.preferred_action);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)title {
    env.objc.borrow::<UIAlertControllerHostObject>(this).title
}

- (id)message {
    env.objc.borrow::<UIAlertControllerHostObject>(this).message
}

- (UIAlertControllerStyle)preferredStyle {
    env.objc.borrow::<UIAlertControllerHostObject>(this).preferred_style
}

- (id)preferredAction {
    env.objc.borrow::<UIAlertControllerHostObject>(this).preferred_action
}

- (())setPreferredAction:(id)action {
    let action = retain(env, action);
    let host = env.objc.borrow_mut::<UIAlertControllerHostObject>(this);
    let old = host.preferred_action;
    host.preferred_action = action;
    release(env, old);
}

- (())addAction:(id)action {
    if action == nil {
        return;
    }
    let actions = env.objc.borrow::<UIAlertControllerHostObject>(this).actions;
    let _: () = msg![env; actions addObject:action];
}

- (id)actions {
    let actions = env.objc.borrow::<UIAlertControllerHostObject>(this).actions;
    if actions == nil {
        return nil;
    }
    let copy: id = msg![env; actions copy];
    crate::objc::autorelease(env, copy)
}

- (())addTextFieldWithConfigurationHandler:(id)handler {
    let text_fields = {
        let host = env.objc.borrow::<UIAlertControllerHostObject>(this);
        host.text_fields
    };
    let text_fields = if text_fields == nil {
        let new_fields: id = msg_class![env; NSMutableArray new];
        env.objc.borrow_mut::<UIAlertControllerHostObject>(this).text_fields = new_fields;
        new_fields
    } else {
        text_fields
    };

    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize { width: 0.0, height: 0.0 },
    };
    let text_field: id = msg_class![env; UITextField alloc];
    let text_field: id = msg![env; text_field initWithFrame:frame];
    let _: () = msg![env; text_fields addObject:text_field];
    if handler != nil {
        invoke_block(env, handler, text_field);
    }
    release(env, text_field);
}

- (id)textFields {
    let text_fields = env.objc.borrow::<UIAlertControllerHostObject>(this).text_fields;
    if text_fields == nil {
        return nil;
    }
    let copy: id = msg![env; text_fields copy];
    crate::objc::autorelease(env, copy)
}

- (())viewDidLoad {
    // No native view hierarchy; presentation is handled by the host dialog.
}

- (())viewDidAppear:(bool)_animated {
    let already_presented = env.objc.borrow::<UIAlertControllerHostObject>(this).presented;
    if already_presented {
        return;
    }
    env.objc.borrow_mut::<UIAlertControllerHostObject>(this).presented = true;

    let (title, message, actions) = {
        let host = env.objc.borrow::<UIAlertControllerHostObject>(this);
        (host.title, host.message, host.actions)
    };

    let count: NSUInteger = if actions == nil {
        0
    } else {
        msg![env; actions count]
    };

    let mut ordered_actions: Vec<id> = Vec::new();
    let mut ordered_titles: Vec<String> = Vec::new();
    let mut cancel_action: id = nil;
    let mut cancel_title: String = String::new();

    for i in 0..count {
        let action: id = msg![env; actions objectAtIndex:i];
        if action == nil {
            continue;
        }
        let style: UIAlertActionStyle = msg![env; action style];
        let title_id: id = msg![env; action title];
        let title_str = if title_id == nil {
            format!("Button {}", i)
        } else {
            ns_string::to_rust_string(env, title_id).into_owned()
        };
        if style == UIAlertActionStyleCancel && cancel_action == nil {
            cancel_action = action;
            cancel_title = title_str;
        } else {
            ordered_actions.push(action);
            ordered_titles.push(title_str);
        }
    }

    if cancel_action != nil {
        ordered_actions.push(cancel_action);
        ordered_titles.push(cancel_title);
    }

    if ordered_actions.is_empty() {
        ordered_titles.push("OK".to_string());
    }

    let title_str = if title == nil {
        "Alert".to_string()
    } else {
        ns_string::to_rust_string(env, title).into_owned()
    };
    let message_str = if message == nil {
        String::new()
    } else {
        ns_string::to_rust_string(env, message).into_owned()
    };

    let clicked = window::show_alert_dialog(
        env,
        &title_str,
        &message_str,
        &ordered_titles.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );

    let selected_action = ordered_actions
        .get(clicked as usize)
        .copied()
        .or_else(|| ordered_actions.first().copied())
        .unwrap_or(nil);
    let selected_action = if selected_action != nil {
        retain(env, selected_action)
    } else {
        nil
    };
    let handler = if selected_action != nil {
        msg![env; selected_action handler]
    } else {
        nil
    };

    let _: () = msg![env; this dismissViewControllerAnimated:false completion:nil];

    if handler != nil {
        invoke_block(env, handler, selected_action);
    }
    if selected_action != nil {
        release(env, selected_action);
    }
}

@end

};
