/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertView`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr, SEL,
};
use std::borrow::Cow;

struct UIAlertViewHostObject {
    superclass: super::UIViewHostObject,
    title: id,
    message: id,
    delegate: id,
    buttons: Vec<id>,
    cancel_button_index: Option<usize>,
}
impl_HostObject_with_superclass!(UIAlertViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIAlertView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::new(UIAlertViewHostObject {
            superclass: Default::default(),
            title: nil,
            message: nil,
            delegate: nil,
            buttons: Vec::new(),
            cancel_button_index: None,
        }),
        &mut env.mem
    )
}

- (id)initWithTitle:(id)title
                      message:(id)message
                     delegate:(id)delegate
            cancelButtonTitle:(id)cancelButtonTitle
            otherButtonTitles:(id)otherButtonTitles {

    let message_text = if message == nil {
        Cow::from("(nil)")
    } else {
        ns_string::to_rust_string(env, message)
    };
    let title_text = if title == nil {
        Cow::from("(nil)")
    } else {
        ns_string::to_rust_string(env, title)
    };
    log!(
        "UIAlertView: title: {:?}, message: {:?}",
        title_text,
        message_text
    );

    let this = msg_super![env; this init];
    retain(env, title);
    retain(env, message);
    retain(env, delegate);
    let cancel_button_index = (cancelButtonTitle != nil).then_some(0);
    if cancelButtonTitle != nil {
        retain(env, cancelButtonTitle);
    }
    if otherButtonTitles != nil {
        retain(env, otherButtonTitles);
    }
    let alert = env.objc.borrow_mut::<UIAlertViewHostObject>(this);
    alert.title = title;
    alert.message = message;
    alert.delegate = delegate;
    alert.cancel_button_index = cancel_button_index;
    if cancelButtonTitle != nil {
        alert.buttons.push(cancelButtonTitle);
    }
    if otherButtonTitles != nil {
        alert.buttons.push(otherButtonTitles);
    }
    this
}

- (())addButtonWithTitle:(id)title {
    retain(env, title);
    env.objc
        .borrow_mut::<UIAlertViewHostObject>(this)
        .buttons
        .push(title);
}

- (())show {
    let (delegate, title_id, message_id, button_ids, cancel_button_index) = {
        let alert = env.objc.borrow::<UIAlertViewHostObject>(this);
        (
            alert.delegate,
            alert.title,
            alert.message,
            alert.buttons.clone(),
            alert.cancel_button_index,
        )
    };
    if delegate == nil {
        return;
    }

    let title = if title_id == nil {
        String::new()
    } else {
        ns_string::to_rust_string(env, title_id).into_owned()
    };
    let message = if message_id == nil {
        String::new()
    } else {
        ns_string::to_rust_string(env, message_id).into_owned()
    };
    let buttons = button_ids
        .iter()
        .map(|button| ns_string::to_rust_string(env, *button).into_owned())
        .collect::<Vec<_>>();

    let button_index = match env.on_parent_stack_in_coroutine(|window, _| {
        crate::window::show_alert_messagebox(
            Some(window),
            &title,
            &message,
            &buttons,
            cancel_button_index,
        )
    }) {
        Ok(-1) => cancel_button_index.map_or(-1, |index| index as i32),
        Ok(index) => index,
        Err(error) => {
            log!("UIAlertView: failed to show native alert: {}", error);
            return;
        }
    };

    let selector: SEL = env.objc.register_host_selector(
        "alertView:clickedButtonAtIndex:".to_string(),
        &mut env.mem,
    );
    let responds: bool = msg![env; delegate respondsToSelector:selector];
    if responds {
        () = msg![env; delegate alertView:this clickedButtonAtIndex:button_index];
    }
}

- (())dealloc {
    let (title, message, delegate, buttons) = {
        let alert = env.objc.borrow::<UIAlertViewHostObject>(this);
        (
            alert.title,
            alert.message,
            alert.delegate,
            alert.buttons.clone(),
        )
    };
    release(env, title);
    release(env, message);
    release(env, delegate);
    for button in buttons {
        release(env, button);
    }
    msg_super![env; this dealloc]
}

@end

};
