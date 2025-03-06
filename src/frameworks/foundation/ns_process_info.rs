/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSProcessInfo`.

use super::{NSTimeInterval, NSUInteger};
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports};
use crate::Environment;
use std::time::Instant;

#[derive(Default)]
pub struct State {
    instance: Option<id>,
}

impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.foundation.ns_process_info
    }
}

// using 2GB; see mocked memory data src/libc/mach_host.rs
// TODO: If mach_host.rs is written better this should be moved
pub const PHYSICAL_MEMORY: u32 = 2 * 1024 * 1024 * 1024;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSProcessInfo: NSObject

+ (id)processInfo {
    if let Some(ns_process_info_instance) = State::get(env).instance {
        return ns_process_info_instance;
    }

    let new_obj: id = msg_class![env; NSProcessInfo alloc];
    let new_obj: id = msg![env; new_obj init];

    State::get(env).instance = Some(new_obj);

    new_obj
}

- (NSTimeInterval)systemUptime {
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

- (NSUInteger)physicalMemory {
    PHYSICAL_MEMORY
}

@end

};
