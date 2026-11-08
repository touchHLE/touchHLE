/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIActionSheet`.

use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::ns_string;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg_super, nil, objc_classes, ClassExports};
use std::borrow::Cow;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIActionSheet: UIView

- (id)initWithTitle:(id)title
            delegate:(id)delegate
   cancelButtonTitle:(id)cancelButtonTitle
destructiveButtonTitle:(id)destructiveButtonTitle
   otherButtonTitles:(id)otherButtonTitles {

    let title = if title == nil { Cow::from("(nil)") } else { ns_string::to_rust_string(env, title) };
    log!("TODO: [(UIActionSheet*){:?} initWithTitle:{:?} delegate:{:?} cancelButtonTitle:{:?} destructiveButtonTitle:{:?} otherButtonTitles:{:?}]", this, title, delegate, cancelButtonTitle, destructiveButtonTitle, otherButtonTitles);

    msg_super![env; this init]
}

- (())addButtonWithTitle:(id)title {
    log!("TODO: [(UIActionSheet *){:?} addButtonWithTitle:{:?}]", this, ns_string::to_rust_string(env, title));
}

- (())showInView:(id)view {
    log!("TODO: [(UIActionSheet*){:?} showInView:{:?}]", this, view);
}

- (())showFromBarButtonItem:(id)item animated:(bool)animated {
    log!("TODO: [(UIActionSheet*){:?} showFromBarButtonItem:{:?} animated:{}]", this, item, animated);
}

- (())showFromRect:(CGRect)rect inView:(id)view animated:(bool)animated {
    log!("TODO: [(UIActionSheet*){:?} showFromRect:{:?} inView:{:?} animated:{}]", this, rect, view, animated);
}

- (())showFromTabBar:(id)tabBar {
    log!("TODO: [(UIActionSheet*){:?} showFromTabBar:{:?}]", this, tabBar);
}

- (())showFromToolbar:(id)toolbar {
    log!("TODO: [(UIActionSheet*){:?} showFromToolbar:{:?}]", this, toolbar);
}

- (())dismissWithClickedButtonIndex:(NSInteger)buttonIndex animated:(bool)animated {
    log!("TODO: [(UIActionSheet*){:?} dismissWithClickedButtonIndex:{} animated:{}]", this, buttonIndex, animated);
}

- (id)title {
    return nil;
}
- (())setTitle:(id)title {
    log!("TODO: [(UIActionSheet*){:?} setTitle:{:?}]", this, ns_string::to_rust_string(env, title));
}

- (id)buttonTitleAtIndex:(NSInteger)index {
    log!("TODO: [(UIActionSheet*){:?} buttonTitleAtIndex:{}]", this, index);
    return nil;
}

- (NSInteger)numberOfButtons {
    return 0;
}

- (NSInteger)cancelButtonIndex {
    return -1;
}
- (())setCancelButtonIndex:(NSInteger)index {
    log!("TODO: [(UIActionSheet*){:?} setCancelButtonIndex:{}]", this, index);
}

- (NSInteger)destructiveButtonIndex {
    return -1;
}
- (())setDestructiveButtonIndex:(NSInteger)index {
    log!("TODO: [(UIActionSheet*){:?} setDestructiveButtonIndex:{}]", this, index);
}

- (NSInteger)firstOtherButtonIndex {
    return -1;
}

- (bool)isVisible {
    return false;
}

@end

};
