/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSProcessInfo`.

use super::{NSTimeInterval, NSUInteger};
use crate::objc::{id, objc_classes, ClassExports};
use std::time::Instant;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSProcessInfo: NSObject

+ (id)processInfo {
    this.cast()
}

+ (NSTimeInterval)systemUptime {
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

+ (NSUInteger)physicalMemory {
    // see mocked memory data src/libc/mach_host.rs
    2 * 1024 * 1024 * 1024 // `2GB`
}

@end

};
