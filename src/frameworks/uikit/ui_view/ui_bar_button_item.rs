/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarButtonItem`.

use crate::{
    frameworks::foundation::NSInteger,
    objc::{id, objc_classes, ClassExports, SEL},
};

type UIBarButtonSystemItem = NSInteger;

pub const CLASSES: ClassExports = objc_classes! {

// TODO: rendering

(env, this, _cmd);

@implementation UIBarButtonItem: UIBarItem
// TODO: add remaining functions

- (())initWithBarButtonSystemItem:(UIBarButtonSystemItem)systemItem
        target:(id)target
        action:(SEL)action {
    log!("[(UIBarButtonItem*){:?} initWithBarButtonSystemItem:{:?}, {:?}, {:?}] TODO: Implement UIBarButtonItem. The control won't be rendered.", this, systemItem, target, action);
}

@end

};
