/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarItem`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, nil, objc_classes, release, retain, ClassExports, HostObject};

pub mod ui_bar_button_item;

pub struct UIBarItemHostObject {
    pub title: id,
    pub enabled: bool,
    pub tag: NSInteger,
}

impl HostObject for UIBarItemHostObject {}

impl Default for UIBarItemHostObject {
    fn default() -> Self {
        Self {
            title: nil,
            enabled: true,
            tag: 0,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// abstract class
@implementation UIBarItem: NSObject

- (id)title {
    env.objc.borrow::<UIBarItemHostObject>(this).title
}

- (())setTitle:(id)title {
    let old = env.objc.borrow::<UIBarItemHostObject>(this).title;
    retain(env, title);
    release(env, old);
    env.objc.borrow_mut::<UIBarItemHostObject>(this).title = title;
}

- (bool)isEnabled {
    env.objc.borrow::<UIBarItemHostObject>(this).enabled
}

- (())setEnabled:(bool)enabled {
    env.objc.borrow_mut::<UIBarItemHostObject>(this).enabled = enabled;
}

- (NSInteger)tag {
    env.objc.borrow::<UIBarItemHostObject>(this).tag
}

- (())setTag:(NSInteger)tag {
    env.objc.borrow_mut::<UIBarItemHostObject>(this).tag = tag;
}

@end

};
