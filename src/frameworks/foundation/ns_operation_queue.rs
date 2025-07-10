/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSOperationQueue`.
//!
//! Resources:
//! - [Apple's Concurrency Programming Guide](https://developer.apple.com/library/archive/documentation/General/Conceptual/ConcurrencyProgrammingGuide/OperationObjects/OperationObjects.html)

use std::collections::VecDeque;

use crate::abi::GuestFunction;
use crate::dyld::HostFunction;
use crate::environment::Environment;
use crate::libc::pthread::cond::{
    pthread_cond_broadcast, pthread_cond_destroy, pthread_cond_init, pthread_cond_t,
    pthread_cond_wait,
};
use crate::libc::pthread::mutex::{
    pthread_mutex_destroy, pthread_mutex_init, pthread_mutex_lock, pthread_mutex_t,
    pthread_mutex_unlock,
};
use crate::libc::semaphore::{
    host_create_semaphore, host_destroy_semaphore, sem_post, sem_t, sem_wait,
};
use crate::mem::{guest_size_of, ConstPtr, MutPtr, MutVoidPtr};
use crate::msg_class;
use crate::objc::{id, msg, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr};

pub struct State {
    helper_guest_func: GuestFunction,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.foundation.ns_operation_queue
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            helper_guest_func: GuestFunction::null_ptr(),
        }
    }
}

struct NSOperationQueueHostObject {
    is_empty_cond: MutPtr<pthread_cond_t>,
    is_empty_mutex: MutPtr<pthread_mutex_t>,
    queued_operations_semaphore: MutPtr<sem_t>,
    queue: VecDeque<id>,
    thread_exists: bool,
}
impl HostObject for NSOperationQueueHostObject {}

impl NSOperationQueueHostObject {
    pub fn new(env: &mut Environment) -> Self {
        let is_empty_cond: MutPtr<pthread_cond_t> =
            env.mem.alloc(guest_size_of::<pthread_cond_t>()).cast();
        let is_empty_mutex = env.mem.alloc(guest_size_of::<pthread_mutex_t>()).cast();
        pthread_mutex_init(env, is_empty_mutex, ConstPtr::null());
        pthread_cond_init(env, is_empty_cond, ConstPtr::null());
        let queued_operations_semaphore = host_create_semaphore(env, 0);
        Self {
            is_empty_cond,
            is_empty_mutex,
            queued_operations_semaphore,
            queue: VecDeque::new(),
            thread_exists: false,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);
@implementation NSOperationQueue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = NSOperationQueueHostObject::new(env);
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (())addOperation:(id)operation {
    let operation = retain(env, operation);
    let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
    if !host_object.thread_exists {
        host_object.thread_exists = true;
        let state = State::get(env);
        let operation_queue_gf = if state.helper_guest_func.to_ptr().is_null() {
            let operation_queue_hf: HostFunction =
                &(operation_queue_thread_helper as fn(&mut Environment, _) -> _);
            let operation_queue_gf = env.dyld.create_guest_function(
                env.mem.as_mut(),
                "__touchHLE_operation_queue_helper",
                operation_queue_hf,
            );
            let state = State::get(env);
            state.helper_guest_func = operation_queue_gf;
            operation_queue_gf
        } else {
            state.helper_guest_func
        };
        // We need to call retain before the thread starts, otherwise we can
        // lose the queue before the thread even begins.
        retain(env, this);
        env.new_thread(
            operation_queue_gf,
            this.cast(),
            crate::mem::Mem::SECONDARY_THREAD_DEFAULT_STACK_SIZE,
        );
    }
    let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
    host_object.queue.push_back(operation);
    let queue_semaphore = host_object.queued_operations_semaphore;
    sem_post(env, queue_semaphore);
}

- (())waitUntilAllOperationsAreFinished {
    let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
    let retained_operations = host_object.queue.clone();
    let mutex = host_object.is_empty_mutex;
    let cond = host_object.is_empty_cond;

    // Retains operations until finished
    for operation in retained_operations.iter() {
        retain(env, *operation);
    }

    pthread_mutex_lock(env, mutex);
    loop {
        let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
        if host_object.queue.is_empty(){
            break
        }
        pthread_cond_wait(env, cond, mutex);
    }
    pthread_mutex_unlock(env, mutex);
    for operation in retained_operations {
        release(env, operation);
    }
}

- (())dealloc {
    let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
    let sem = host_object.queued_operations_semaphore;
    let mutex = host_object.is_empty_mutex;
    let cond = host_object.is_empty_cond;
    let queue = std::mem::take(&mut host_object.queue);
    host_destroy_semaphore(env, sem);
    pthread_mutex_destroy(env, mutex);
    pthread_cond_destroy(env, cond);
    for oper in queue {
        release(env, oper);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end
};

fn operation_queue_thread_helper(env: &mut Environment, data: MutVoidPtr) -> MutVoidPtr {
    let this: id = data.cast();
    let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
    let sem = host_object.queued_operations_semaphore;
    // Using get_refcount is usually not the best idea, but we only care about
    // the case that we're the only remaining retain. (This can break down if
    // apps are releasing then immediately retaining the object, but hopefully
    // that is not an issue!)
    //
    // This is also (seemingly?) how apple handles this, so I think it's ok.
    while env.objc.get_refcount(this).get() != 1
        || !env
            .objc
            .borrow::<NSOperationQueueHostObject>(this)
            .queue
            .is_empty()
    {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        sem_wait(env, sem);
        let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
        let next_oper = host_object.queue.pop_front().unwrap();
        let _: () = msg![env; next_oper main];
        release(env, next_oper);
        let _: () = msg![env; pool drain];
        let host_object = env.objc.borrow_mut::<NSOperationQueueHostObject>(this);
        if host_object.queue.is_empty() {
            let mutex = host_object.is_empty_mutex;
            let cond = host_object.is_empty_cond;
            pthread_mutex_lock(env, mutex);
            pthread_cond_broadcast(env, cond);
            pthread_mutex_unlock(env, mutex);
        }
    }
    release(env, this);
    MutVoidPtr::null()
}
