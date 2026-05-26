/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIToolbar`.

use crate::objc::{id, objc_classes, ClassExports, nil};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIToolbar: UIView

- (())setItems:(id)_items // NSArray<UIBarButtonItem*>*
    animated:(bool)_animated {
    // TODO
}

- (id)items {
    // TODO
    nil
}

- (())setBarStyle:(i32)_style { // UIBarStyle
    // TODO
}

- (())setTranslucent:(bool)_translucent {
    // TODO
}

- (())setTintColor:(id)_color { // UIColor*
    // TODO
}

- (())setBarTintColor:(id)_color { // UIColor*
    // TODO
}

- (())setDelegate:(id)_delegate {
    // TODO
}

- (())sizeToFit {
    // TODO
}

@end

};
