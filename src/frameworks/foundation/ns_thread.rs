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
    pthread_attr_init, pthread_attr_setdetachstate, pthread_attr_t, pthread_create, pthread_t,
    PTHREAD_CREATE_DETACHED,
};
use crate::mem::{guest_size_of, MutPtr};
use crate::objc::{
    id, msg_send, nil, objc_classes, release, retain, Class, ClassExports, HostObject, NSZonePtr,
    SEL,
};
use crate::Environment;
use crate::{msg, msg_class};
use std::time::Duration;

struct NSThreadHostObject {
    target: id,
    selector: Option<SEL>,
    object: id,
    /// `NSMutableDictionary*`
    thread_dictionary: id,
}
impl HostObject for NSThreadHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSThread: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSThreadHostObject {
        target: nil,
        selector: None,
        object: nil,
        thread_dictionary: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
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
    // Simple hack to make the `setThreadPriority:` work as an instance method
    // (it's both a class and an instance method). Must be replaced if we ever
    // need to support other methods.
    this
}

+ (())sleepForTimeInterval:(NSTimeInterval)ti {
    log_dbg!("[NSThread sleepForTimeInterval:{:?}]", ti);
    env.sleep(Duration::from_secs_f64(ti), /* tail_call: */ true);
}

+ (())detachNewThreadSelector:(SEL)selector
                       toTarget:(id)target
                     withObject:(id)object {
    let host_object = Box::new(NSThreadHostObject {
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

// TODO: construction etc
- (id)threadDictionary {
    // Initialize lazily in case the thread is started with pthread_create
    let thread_dictionary = env.objc.borrow::<NSThreadHostObject>(this).thread_dictionary;
    if thread_dictionary == nil {
        let thread_dictionary = msg_class![env; NSMutableDictionary new];
        // TODO: Store the thread's default NSConnection
        // and NSAssertionHandler instances
        // https://developer.apple.com/documentation/foundation/nsthread/1411433-threaddictionary
        env.objc.borrow_mut::<NSThreadHostObject>(this).thread_dictionary = thread_dictionary;
        thread_dictionary
    } else {
        thread_dictionary
    }
}

- (())dealloc {
    log_dbg!("[(NSThread*){:?} dealloc]", this);
    let host_object = env.objc.borrow::<NSThreadHostObject>(this);
    release(env, host_object.thread_dictionary);
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
