/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSThread`.

use crate::dyld::FunctionExports;
use crate::environment::Environment;
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
use crate::{export_c_func, msg};

struct NSThreadHostObject {
    target: id,
    selector: Option<SEL>,
    object: id,
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

+ (())detachNewThreadSelector:(SEL)selector
                       toTarget:(id)target
                     withObject:(id)object {
    let host_object = Box::new(NSThreadHostObject {
        target,
        selector: Some(selector),
        object,
    });
    let this = env.objc.alloc_object(this, host_object, &mut env.mem);
    retain(env, this);

    retain(env, target);
    retain(env, object);

    let symb = "__ns_thread_invocation";
    let gf = env
        .dyld
        .create_private_proc_address(&mut env.mem, &mut env.cpu, symb)
        .unwrap_or_else(|_| panic!("create_private_proc_address failed {}", symb));

    let attr: MutPtr<pthread_attr_t> = env.mem.alloc(guest_size_of::<pthread_attr_t>()).cast();
    pthread_attr_init(env, attr);

    pthread_attr_setdetachstate(env, attr, PTHREAD_CREATE_DETACHED);
    let thread_ptr: MutPtr<pthread_t> = env.mem.alloc(guest_size_of::<pthread_t>()).cast();

    pthread_create(env, thread_ptr, attr.cast_const(), gf, this.cast());

    // TODO: post NSWillBecomeMultiThreadedNotification
}

// TODO: construction etc

@end

};

type NSThreadRef = CFTypeRef;

pub fn _ns_thread_invocation(env: &mut Environment, ns_thread_obj: NSThreadRef) {
    let class: Class = msg![env; ns_thread_obj class];
    log_dbg!(
        "_ns_thread_invocation on object of class: {}",
        env.objc.get_class_name(class)
    );
    assert_eq!(class, env.objc.get_known_class("NSThread", &mut env.mem));

    let &NSThreadHostObject {
        target,
        selector,
        object,
    } = env.objc.borrow(ns_thread_obj);
    () = msg_send(env, (target, selector.unwrap(), object));

    release(env, object);
    release(env, target);

    release(env, ns_thread_obj);

    // TODO: NSThread exit
}

pub const PRIVATE_FUNCTIONS: FunctionExports = &[export_c_func!(_ns_thread_invocation(_))];
