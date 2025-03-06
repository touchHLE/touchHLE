/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSProcessInfo`.

use super::{NSTimeInterval, NSUInteger};
use crate::objc::{id, objc_classes, ClassExports};
use std::time::Instant;

// using 2GB; see mocked memory data src/libc/mach_host.rs
pub const PHYSICAL_MEMORY: u32 = 2 * 1024 * 1024 * 1024;
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSProcessInfo: NSObject

// Class method returning the shared singleton instance.
// Equivalent to +[NSProcessInfo processInfo].
+ (id)processInfo {
    this.cast()
}

// systemUptime is typically an *instance* method returning uptime
// since boot in seconds. However, to match real-world app usage,
// we also expose it as a class method.
// This handles cases where apps store the class object
// instead of the singleton and then call systemUptime on it.
+ (NSTimeInterval)systemUptime {
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

// physicalMemory is typically an *instance* method
// However, it has the same issue as sytemUptime
// So, we must provide it as a class method too
+ (NSUInteger)physicalMemory {
    PHYSICAL_MEMORY
}

// Correct, instance version of systemUptime.
- (NSTimeInterval)systemUptime {
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

// Correct, instance verson of physicalMemory
- (NSUInteger)physicalMemory {
    PHYSICAL_MEMORY
}

@end

};
