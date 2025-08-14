/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWindow`.

use super::UIViewHostObject;
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::CGRect;
use crate::objc::{id, msg, msg_class, msg_super, nil, objc_classes, ClassExports};

#[derive(Default)]
pub struct State {
    /// List of visible windows for internal purposes. Non-retaining!
    ///
    /// This is public because Core Animation also uses it.
    pub visible_windows: Vec<id>,
    /// The most recent window which received `makeKeyAndVisible` message.
    /// Non-retaining!
    pub key_window: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWindow: UIView

// TODO: more?

- (id)initWithFrame:(CGRect)frame {
    // setHidden: may get called during the super call and panics if the window
    // is not in the list, so it must be added to it before that call.
    let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
    visible_list.push(this);
    log_dbg!(
        "New window: {:?}. New set of visible windows: {:?}",
        this,
        visible_list,
    );

    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    // setHidden: may get called during the super call and panics if the window
    // is not in the list, so it must be added to it before that call.
    let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
    visible_list.push(this);
    log_dbg!(
        "New window: {:?}. New set of visible windows: {:?}",
        this,
        visible_list,
    );

    msg_super![env; this initWithCoder:coder]
}

- (())dealloc {
    if let Some(key_window) = env.framework_state.uikit.ui_view.ui_window.key_window {
        if key_window == this {
            env.framework_state.uikit.ui_view.ui_window.key_window = None;
        }
    }
    if !msg![env; this isHidden] {
        let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
        let idx = visible_list.iter().position(|&w| w == this).unwrap();
        visible_list.remove(idx);
        log_dbg!(
            "Deallocating window {:?}. New set of visible windows: {:?}",
            this,
            visible_list,
        );
    }
    msg_super![env; this dealloc]
}

- (())setHidden:(bool)is_hidden {
    let was_hidden: bool = msg![env; this isHidden];
    () = msg_super![env; this setHidden:is_hidden];

    let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
    if is_hidden && !was_hidden {
        let idx = visible_list.iter().position(|&w| w == this).unwrap();
        visible_list.remove(idx);
        log_dbg!(
            "Window {:?} is now hidden. New set of visible windows: {:?}",
            this,
            visible_list,
        );
    } else if !is_hidden && was_hidden {
        visible_list.push(this);
        log_dbg!(
            "Window {:?} is no longer hidden. New set of visible windows: {:?}",
            this,
            visible_list,
        );
    }
}

- (())makeKeyAndVisible {
    // TODO: We don't currently have send any non-touch events to windows,
    // so there's no meaning in it yet.

    env.framework_state.uikit.ui_view.ui_window.key_window = Some(this);

    msg![env; this setHidden:false]
}

// UIResponder implementation
// From the Apple UIView docs regarding [UIResponder nextResponder]:
// "UIWindow returns the application object."
- (id)nextResponder {
    msg_class![env; UIApplication sharedApplication]
}

- (())addSubview:(id)view {
    log_dbg!("[(UIWindow*){:?} addSubview:{:?}] => ()", this, view);

    if view == nil || env.objc.borrow::<UIViewHostObject>(view).view_controller == nil {
        () = msg_super![env; this addSubview:view];
        return;
    }

    // Below we treat a special case of adding view controller's view
    // to a window, in order to generate display related notifications

    if env.objc.borrow::<UIViewHostObject>(this).subviews.contains(&view) {
        // For the case of existing view hidden by another view,
        // we need to delay a below sequence up until obstructions are removed
        log!("TODO: case of existing view hidden by another view for sending view[Will,Did]Appear");
    }

    let vc = env.objc.borrow::<UIViewHostObject>(view).view_controller;
    () = msg![env; vc viewWillAppear:false];
    () = msg_super![env; this addSubview:view];
    () = msg![env; vc viewDidAppear:false];
}

@end

};

// TODO: more keyboard notifications
pub const UIKeyboardWillShowNotification: &str = "UIKeyboardWillShowNotification";
pub const UIKeyboardDidShowNotification: &str = "UIKeyboardDidShowNotification";
pub const UIKeyboardWillHideNotification: &str = "UIKeyboardWillHideNotification";
pub const UIKeyboardDidHideNotification: &str = "UIKeyboardDidHideNotification";
pub const UIKeyboardBoundsUserInfoKey: &str = "UIKeyboardBoundsUserInfoKey";

pub const CONSTANTS: ConstantExports = &[
    (
        "_UIKeyboardWillShowNotification",
        HostConstant::NSString(UIKeyboardWillShowNotification),
    ),
    (
        "_UIKeyboardDidShowNotification",
        HostConstant::NSString(UIKeyboardDidShowNotification),
    ),
    (
        "_UIKeyboardWillHideNotification",
        HostConstant::NSString(UIKeyboardWillHideNotification),
    ),
    (
        "_UIKeyboardDidHideNotification",
        HostConstant::NSString(UIKeyboardDidHideNotification),
    ),
    (
        "_UIKeyboardBoundsUserInfoKey",
        HostConstant::NSString(UIKeyboardBoundsUserInfoKey),
    ),
];
