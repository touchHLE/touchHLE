/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `GKLocalPlayer`.

use touchhle_macros::validate_class_exports;

use crate::dyld::{ConstantExports, HostConstant};
use crate::objc::{objc_classes, ClassExports};

#[validate_class_exports]
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// TODO: proper inheritance chain
@implementation GKLocalPlayer: NSObject
// TODO
@end

};

pub const GKPlayerAuthenticationDidChangeNotificationName: &str =
    "GKPlayerAuthenticationDidChangeNotificationName";

/// `NSNotificationName` values.
pub const CONSTANTS: ConstantExports = &[(
    "_GKPlayerAuthenticationDidChangeNotificationName",
    HostConstant::NSString(GKPlayerAuthenticationDidChangeNotificationName),
)];
