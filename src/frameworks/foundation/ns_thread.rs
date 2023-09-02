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
use crate::libc::pthread::thread::{_get_thread_id, pthread_t};
use crate::objc::{id, nil, objc_classes, ClassExports, HostObject, SEL};

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

+ (id)alloc {
    log_dbg!("[NSThread alloc]");
    let host_object = NSThreadHostObject { thread: None, target: nil, selector: None, object: nil, thread_dictionary: nil };
    let guest_object = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    State::get(env).ns_threads.insert(guest_object);
    guest_object
}

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

- (id)initWithTarget:(id)target
selector:(SEL)selector
object:(id)object {
    let host_object: &mut NSThreadHostObject = env.objc.borrow_mut(this);
    host_object.target = target;
    host_object.selector = Some(selector);
    host_object.object = object;
    this
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

- (())dealloc {
    log_dbg!("[(NSThread*){:?} dealloc]", this);
    State::get(env).ns_threads.remove(&this);
    let _host_object = env.objc.borrow::<NSThreadHostObject>(this);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
