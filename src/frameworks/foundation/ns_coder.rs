/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCoder`.

use touchhle_macros::validate_class_exports;

use crate::objc::{objc_classes, ClassExports};

#[validate_class_exports]
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSCoder: NSObject
// This is an abstract class
@end

};
