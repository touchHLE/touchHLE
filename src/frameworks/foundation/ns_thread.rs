/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSThread`.

use crate::objc::{objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSThread: NSObject

+ (f64)threadPriority {
    log!("TODO: [NSThread threadPriority] (not implemented yet)");
    1.0
}

+ (bool)setThreadPriority:(f64)priority {
    log!("TODO: [NSThread setThreadPriority:{:?}] (ignored)", priority);
    true
}

// TODO: construction etc

@end

};
