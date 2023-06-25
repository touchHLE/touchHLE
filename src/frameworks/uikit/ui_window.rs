/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWindow`.

use crate::objc::{msg, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWindow: UIView

// TODO

- (())makeKeyAndVisible {
    // TODO: Set the "key" window once it's relevant. We don't currently have
    // send any non-touch events to windows, so there's no meaning in it yet.

    msg![env; this setHidden:false]
}

@end

};
