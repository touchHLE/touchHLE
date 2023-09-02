/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSThread`.

use std::collections::HashSet;
use std::time::Duration;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::frameworks::core_foundation::CFTypeRef;
use crate::frameworks::foundation::NSTimeInterval;
use crate::libc::pthread::thread::{_get_thread_id, pthread_create, pthread_t};
use crate::mem::{guest_size_of, ConstPtr, MutPtr};
use crate::objc::{id, msg_send, nil, objc_classes, Class, ClassExports, HostObject, SEL};
use crate::{export_c_func, msg, msg_class};

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

- (())start {
    let symb = "__ns_thread_invocation";
    let gf = env
        .dyld
        .create_proc_address(&mut env.mem, &mut env.cpu, symb)
        .unwrap_or_else(|_| panic!("create_proc_address failed {}", symb));

    let thread_ptr: MutPtr<pthread_t> = env.mem.alloc(guest_size_of::<pthread_t>()).cast();
    pthread_create(env, thread_ptr, ConstPtr::null(), gf, this.cast());
    let thread = env.mem.read(thread_ptr);
    let thread_dictionary = msg_class![env; NSDictionary alloc];
    // TODO: Store the thread's default NSConnection and NSAssertionHandler instances
    // https://developer.apple.com/documentation/foundation/nsthread/1411433-threaddictionary

    let host_object = env.objc.borrow_mut::<NSThreadHostObject>(this);
    host_object.thread = Some(thread);
    host_object.thread_dictionary = thread_dictionary;

    log_dbg!("[(NSThread*){:?} start] Started new thread with pthread {:?} and ThreadId {:?}", this, thread, _get_thread_id(env, thread));
}

- (id)threadDictionary {
    env.objc.borrow::<NSThreadHostObject>(this).thread_dictionary
}

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
    let host_object = env.objc.borrow::<NSThreadHostObject>(this);
    if !host_object.thread_dictionary.is_null() {
        env.mem.free(host_object.thread_dictionary.cast());
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

type NSThreadRef = CFTypeRef;

pub fn _ns_thread_invocation(env: &mut Environment, ns_thread_obj: NSThreadRef) {
    let class: Class = msg![env; ns_thread_obj class];
    assert_eq!(class, env.objc.get_known_class("NSThread", &mut env.mem));

    let &NSThreadHostObject {
        target,
        selector,
        object,
        ..
    } = env.objc.borrow(ns_thread_obj);
    () = msg_send(env, (target, selector.unwrap(), object));
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(_ns_thread_invocation(_))];
