/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGPath`
use crate::dyld::FunctionExports;
use crate::objc::{objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGMutablePath: NSObject
@end

};

pub type CGLineCap = i32;
pub const kCGLineCapButt: CGLineCap = 0; // Default
pub const kCGLineCapRound: CGLineCap = 1;
pub const kCGLineCapSquare: CGLineCap = 2;

pub const FUNCTIONS: FunctionExports = &[];
