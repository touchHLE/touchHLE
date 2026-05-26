/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarButtonItem`.

use crate::objc::{id, objc_classes, ClassExports, SEL};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIBarButtonItem: NSObject

- (id)initWithBarButtonSystemItem:(i32)system_item
                           target:(id)target
                           action:(SEL)action {
    log!("TODO: [(UIBarButtonItem*){:?} initWithBarButtonSystemItem:{} target:{:?} action:{:?}]", this, system_item, target, action);
    this
}

- (id)initWithImage:(id)image
              style:(i32)style
             target:(id)target
             action:(SEL)action {
    log!("TODO: [(UIBarButtonItem*){:?} initWithImage:{:?} style:{} target:{:?} action:{:?}]", this, image, style, target, action);
    this
}

- (id)initWithTitle:(id)title
              style:(i32)style
             target:(id)target
             action:(SEL)action {
    log!("TODO: [(UIBarButtonItem*){:?} initWithTitle:{:?} style:{} target:{:?} action:{:?}]", this, title, style, target, action);
    this
}

- (())setEnabled:(bool)enabled {
    log!("TODO: [(UIBarButtonItem*){:?} setEnabled:{}]", this, enabled);
}

@end

};
