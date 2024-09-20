/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPickerView`.

use crate::objc::{id, objc_classes, ClassExports};

// TODO: rendering

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPickerView: UIView

- (())setShowsSelectionIndicator:(bool)shows {
    log!("TODO: [(UIPickerView*){:?} setShowsSelectionIndicator:{}]", this, shows);
}
- (())setDelegate:(id)_delegate {
    // TODO
}

@end

};
