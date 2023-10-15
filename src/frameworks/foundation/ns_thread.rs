/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSThread`.

use std::collections::HashMap;
use crate::abi::GuestFunction;
use crate::environment::{Environment, ThreadId};
use crate::frameworks::foundation::NSUInteger;
use crate::{export_c_func, msg, msg_class};
use crate::dyld::FunctionExports;
use crate::libc::pthread::thread::pthread_create_inner;
use crate::mem::{ConstPtr, Mem, MutPtr};
use crate::objc::{id, objc_classes, ClassExports, HostObject, SEL, release, retain, nil, NSZonePtr, msg_send};

#[derive(Default)]
pub struct State {
    thread_start_fn: Option<GuestFunction>,
    thread_map: HashMap<ThreadId, id>
}

/// Belongs to NSThread
struct ThreadHostObject {
    target: id,
    selector: SEL,
    argument: id,
    stack_size: NSUInteger,
    run_loop: id,
    is_cancelled: bool
}
impl HostObject for ThreadHostObject {}
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


+ (id)mainThread {
    if !env.framework_state.foundation.ns_thread.thread_map.contains_key(&0) {
        let thread = msg![env; this alloc];
        let r_loop = env.objc.borrow_mut::<ThreadHostObject>(thread).run_loop;
        () = msg![env; r_loop _setMainThread];
        env.framework_state.foundation.ns_thread.thread_map.insert(0, thread);
    }
    *env.framework_state.foundation.ns_thread.thread_map.get(&0).unwrap()
}

+ (id)currentThread {
    if env.current_thread == 0 {
        msg![env; this mainThread]
    } else {
        *env.framework_state.foundation.ns_thread.thread_map.get(&env.current_thread).unwrap()
    }
}

+ (id)allocWithZone:(NSZonePtr)_zone {
    let r_loop = msg_class![env; NSRunLoop alloc];
    let host_object = Box::new(ThreadHostObject {
        target: nil,
        argument: nil,
        selector: SEL::null(),
        stack_size: Mem::SECONDARY_THREAD_STACK_SIZE,
        run_loop: r_loop,
        is_cancelled: false
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTarget:(id)target
            selector:(SEL)selector
            object:(id)argument {
    retain(env, target);
    retain(env, argument);
    let host = env.objc.borrow_mut::<ThreadHostObject>(this);
    host.argument = argument;
    host.target = target;
    host.selector = selector;
    this
}

-(())start {
    retain(env, this); // Balanced with a release at the end of thread fn
    let start_fn = *env.framework_state.foundation.ns_thread.thread_start_fn.get_or_insert_with(|| {
        env.dyld.create_proc_address(&mut env.mem, &mut env.cpu, "__NSThreadStart").unwrap()
    });
    let id = pthread_create_inner(env, MutPtr::null(), ConstPtr::null(), start_fn, this.cast()).1;
    env.framework_state.foundation.ns_thread.thread_map.insert(id, this);
}

-(())main {
    let host = env.objc.borrow::<ThreadHostObject>(this);
    () = msg_send(env, (host.target, host.selector, host.argument));
}

- (f64)threadPriority {
    log!("TODO: [NSThread threadPriority] (not implemented yet)");
    1.0
}

- (bool)setThreadPriority:(f64)priority {
    log!("TODO: [NSThread setThreadPriority:{:?}] (ignored)", priority);
    true
}

-(NSUInteger) stackSize {
    env.objc.borrow::<ThreadHostObject>(this).stack_size
}

-(())setStackSize:(NSUInteger)size {
    env.objc.borrow_mut::<ThreadHostObject>(this).stack_size = size;
}

-(bool) isCancelled {
    env.objc.borrow::<ThreadHostObject>(this).is_cancelled
}

- (())dealloc {
    let &ThreadHostObject{argument, target, run_loop, ..} = env.objc.borrow(this);
    release(env, argument);
    release(env, target);
    release(env, run_loop);

    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: construction etc

@end

};

pub fn get_run_loop(env: &mut Environment, thread: id) -> id {
    env.objc.borrow::<ThreadHostObject>(thread).run_loop
}

fn _NSThreadStart(env: &mut Environment, thread: id) {
    () = msg![env; thread main];
    release(env, thread);
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(_NSThreadStart(_))];
