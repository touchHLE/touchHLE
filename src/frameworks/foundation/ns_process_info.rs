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
// TODO: If mach_host.rs is written better this should be moved
pub const PHYSICAL_MEMORY: u32 = 2 * 1024 * 1024 * 1024;
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSProcessInfo: NSObject

// TODO: Fix this implentation so that it correctly uses instances
// touchHLE impliments NSProcessInfo in a hacky way
// There is no instance of the class returned, just the class itself
// So methods are implimented as class methods not instance methods

+ (id)processInfo {
    this.cast()
}

// systemUptime is supposed to be an *instance* method
// returning uptime since boot in seconds.
+ (NSTimeInterval)systemUptime {
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

// physicalMemory is supposed to be an *instance* method
+ (NSUInteger)physicalMemory {
    PHYSICAL_MEMORY
}

@end

};
