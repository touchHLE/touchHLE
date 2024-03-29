/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSThread`.

use super::NSTimeInterval;
use crate::dyld::HostFunction;
use crate::frameworks::core_foundation::CFTypeRef;
use crate::libc::pthread::thread::{
    _get_thread_by_id, _get_thread_id, pthread_attr_init, pthread_attr_setdetachstate,
    pthread_attr_t, pthread_create, pthread_t, PTHREAD_CREATE_DETACHED,
};
use crate::mem::{guest_size_of, MutPtr};
use crate::objc::{
    id, msg_send, nil, objc_classes, release, retain, Class, ClassExports, HostObject, NSZonePtr,
    SEL,
};
use crate::Environment;
use crate::{msg, msg_class};
use std::collections::HashSet;
use std::time::Duration;

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

+ (id)allocWithZone:(NSZonePtr)zone {
    log_dbg!("[NSThread allocWithZone:{:?}]", zone);
    let host_object = NSThreadHostObject { thread: None, target: nil, selector: None, object: nil, thread_dictionary: nil };
    let guest_object = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    State::get(env).ns_threads.insert(guest_object);
    guest_object
}

+ (f64)threadPriority {
    let current_thread = msg_class![env; NSThread currentThread];
    msg![env; current_thread threadPriority]
}

+ (bool)setThreadPriority:(f64)priority {
    let current_thread = msg_class![env; NSThread currentThread];
    msg![env; current_thread setThreadPriority:priority]
}

+ (id)currentThread {
    log_dbg!("[NSThread currentThread] (env.current_thread == {:?})",env.current_thread);
    State::get(env).ns_threads.clone().iter().find(|ns_thread| {
        let host_object = env.objc.borrow::<NSThreadHostObject>(**ns_thread);
        match host_object.thread {
            Some(thread) => _get_thread_id(env, thread).unwrap() == env.current_thread,
            None => false
        }
    }).map(|ns_thread| *ns_thread).unwrap_or_else(|| {
        // Handles the case the thread was created with pthread_create but has
        // no corresponding NSThread instance yet.
        let current_ns_thread = msg_class![env; NSThread alloc];
        env.objc.borrow_mut::<NSThreadHostObject>(current_ns_thread).thread = _get_thread_by_id(env, env.current_thread);
        current_ns_thread
    })
}

+ (())sleepForTimeInterval:(NSTimeInterval)ti {
    log_dbg!("[NSThread sleepForTimeInterval:{:?}]", ti);
    env.sleep(Duration::from_secs_f64(ti), /* tail_call: */ true);
}

+ (())detachNewThreadSelector:(SEL)selector
                       toTarget:(id)target
                     withObject:(id)object {
    let host_object = Box::new(NSThreadHostObject {
        thread: None,
        target,
        selector: Some(selector),
        object,
        thread_dictionary: nil,
    });
    let this = env.objc.alloc_object(this, host_object, &mut env.mem);
    retain(env, this);

    retain(env, target);
    retain(env, object);

    let symb = "__touchHLE_NSThreadInvocationHelper";
    let hf: HostFunction = &(_touchHLE_NSThreadInvocationHelper as fn(&mut Environment, _) -> _);
    let gf = env
        .dyld
        .create_guest_function(&mut env.mem, symb, hf);

    let attr: MutPtr<pthread_attr_t> = env.mem.alloc(guest_size_of::<pthread_attr_t>()).cast();
    pthread_attr_init(env, attr);

    pthread_attr_setdetachstate(env, attr, PTHREAD_CREATE_DETACHED);
    let thread_ptr: MutPtr<pthread_t> = env.mem.alloc(guest_size_of::<pthread_t>()).cast();

    pthread_create(env, thread_ptr, attr.cast_const(), gf, this.cast());

    // TODO: post NSWillBecomeMultiThreadedNotification
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

type NSThreadRef = CFTypeRef;

pub fn _touchHLE_NSThreadInvocationHelper(env: &mut Environment, ns_thread_obj: NSThreadRef) {
    let class: Class = msg![env; ns_thread_obj class];
    log_dbg!(
        "_touchHLE_NSThreadInvocationHelper on object of class: {}",
        env.objc.get_class_name(class)
    );
    assert_eq!(class, env.objc.get_known_class("NSThread", &mut env.mem));

    let &NSThreadHostObject {
        thread: _,
        target,
        selector,
        object,
        thread_dictionary: _,
    } = env.objc.borrow(ns_thread_obj);
    () = msg_send(env, (target, selector.unwrap(), object));

    release(env, object);
    release(env, target);

    release(env, ns_thread_obj);

    // TODO: NSThread exit
}
