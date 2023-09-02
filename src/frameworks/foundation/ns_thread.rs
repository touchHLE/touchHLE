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
    log_dbg!("[NSThread currentThread] (env.current_thread == {:?})",env.current_thread);
    // TODO: Don't clone
    *State::get(env).ns_threads.clone().iter().find(|ns_thread| {
        let host_object = env.objc.borrow::<NSThreadHostObject>(**ns_thread);
        match host_object.thread {
            Some(thread) => _get_thread_id(env, thread).unwrap() == env.current_thread,
            None => false
        }
    }).unwrap()
}

+ (())sleepForTimeInterval:(NSTimeInterval)ti {
    log_dbg!("[NSThread sleepForTimeInterval:{:?}]", ti);
    env.sleep(Duration::from_secs_f64(ti), /* tail_call: */ true);
}

// TODO: construction etc
- (f64)threadPriority {
    log!("TODO: [(NSThread*){:?} threadPriority] (not implemented yet)", this);
    1.0
}

- (bool)setThreadPriority:(f64)priority {
    log!("TODO: [(NSThread*){:?} setThreadPriority:{:?}] (ignored)", this, priority);
    true
}

@end

};
