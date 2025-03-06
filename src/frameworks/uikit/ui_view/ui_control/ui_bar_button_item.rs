/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarButtonItem`.

use crate::objc::{id, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

    (env, this, _cmd);

    @implementation UIBarButtonItem: UIControl

    - (())alloc:(id)_text { // NSString*
        // TODO
    }

    - (())initWithTitle:(id)title
                      style:(id)style
                      target:(id)target
                      action:(id)action
    {
        log!("TODO: [(UIBarButtonItem*){:?} initWithTitle:{:?} style:{:?} target:{:?} action:{:?}]", this, title, style, target, action);
    }

    - (())initWithBarButtonSystemItem:(id)title
    target:(id)target
    action:(id)action
    {
        log!("TODO: [(UIBarButtonItem*){:?} initWithTitle:{:?} target:{:?} action:{:?}]", this, title, target, action);
    }

    @end

};
