/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPasteboard`.

use crate::dyld::{ConstantExports, HostConstant};
use crate::objc::{objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPasteboard: NSObject
// TODO
@end

};

/// `UIPasteboard` notification names and userInfo keys (`UIPasteboard.h`).
pub const CONSTANTS: ConstantExports = &[
    (
        "_UIPasteboardChangedNotification",
        HostConstant::NSString("UIPasteboardChangedNotification"),
    ),
    (
        "_UIPasteboardChangedTypesAddedKey",
        HostConstant::NSString("UIPasteboardChangedTypesAddedKey"),
    ),
];
