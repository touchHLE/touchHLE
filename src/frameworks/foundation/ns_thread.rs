/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSThread`.

use std::collections::HashSet;
use std::time::Duration;

use crate::environment::Environment;
use crate::frameworks::foundation::NSTimeInterval;
use crate::libc::pthread::thread::pthread_t;
use crate::objc::{id, objc_classes, ClassExports, HostObject, SEL};

#[derive(Default)]
pub struct State {
    /// `NSThread*`
    ns_threads: HashSet<id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_threads
    }
}

struct NSThreadHostObject {
    thread: Option<pthread_t>,
    target: id,
    selector: Option<SEL>,
    object: id,
    /// `NSDictionary*`
    thread_dictionary: id,
}
impl HostObject for NSThreadHostObject {}

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

+ (id)currentThread {
    // Simple hack to make the `setThreadPriority:` work as an instance method
    // (it's both a class and an instance method). Must be replaced if we ever
    // need to support other methods.
    this
}

+ (())sleepForTimeInterval:(NSTimeInterval)ti {
    log_dbg!("[NSThread sleepForTimeInterval:{:?}]", ti);
    env.sleep(Duration::from_secs_f64(ti), /* tail_call: */ true);
}

// TODO: construction etc

@end

};
