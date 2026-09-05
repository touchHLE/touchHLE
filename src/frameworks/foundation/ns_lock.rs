/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSLock family`.
//!
//! TODO: There is probably an opportunity to refactor common methods.
//! (Need to find a good way to do so! Common super class wouldn't
//! work as it breaks expected inheritance chain.)

use crate::environment::MutexType::PTHREAD_MUTEX_RECURSIVE;
use crate::environment::{MutexId, PTHREAD_MUTEX_DEFAULT};
use crate::frameworks::foundation::NSInteger;
use crate::libc::pthread::cond::{
    pthread_cond_broadcast, pthread_cond_destroy, pthread_cond_init, pthread_cond_t,
    pthread_cond_wait,
};
use crate::libc::pthread::mutex::{
    pthread_mutex_destroy, pthread_mutex_init, pthread_mutex_lock, pthread_mutex_t,
    pthread_mutex_trylock, pthread_mutex_unlock,
};
use crate::mem::{guest_size_of, ConstPtr, MutPtr};
use crate::objc::{id, msg, nil, objc_classes, release, ClassExports, HostObject};

struct NSLockHostObject {
    mutex_id: MutexId,
    name: id,
}
impl HostObject for NSLockHostObject {}

struct NSConditionLockHostObject {
    // TODO: use mutex_id instead?
    mutex: MutPtr<pthread_mutex_t>,
    cond: MutPtr<pthread_cond_t>,
    condition: NSInteger,
    name: id,
}
impl HostObject for NSConditionLockHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSLock: NSObject

+ (id)alloc {
    log_dbg!("[NSLock alloc]");
    let mutex_id = env.mutex_state.init_mutex(PTHREAD_MUTEX_DEFAULT);
    let host_object = NSLockHostObject { mutex_id, name: nil };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

// NSLocking protocol implementation
- (())lock {
    log_dbg!("[(NSLock *){:?} lock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.lock_mutex(host_object.mutex_id).unwrap();
}
- (())unlock {
    log_dbg!("[(NSLock *){:?} unlock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if !env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        echo!("*** -[NSLock unlock]: lock (<NSLock: {:?}> '{:?}') unlocked when not locked", this, host_object.name);
    }
    env.unlock_mutex(host_object.mutex_id).unwrap();
}

- (bool)tryLock {
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        false
    } else {
        env.lock_mutex(host_object.mutex_id).is_ok()
    }
}

- (())setName:(id)name { // NSString *
    let old_name = env.objc.borrow::<NSLockHostObject>(this).name;
    // @property(copy), name has to be copied
    env.objc.borrow_mut::<NSLockHostObject>(this).name = msg![env; name copy];
    release(env, old_name);
}
- (id)name {
    env.objc.borrow::<NSLockHostObject>(this).name
}

- (())dealloc {
    log_dbg!("[(NSLock *){:?} dealloc]", this);
    release(env, env.objc.borrow::<NSLockHostObject>(this).name);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.mutex_state.destroy_mutex(host_object.mutex_id).unwrap();
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation NSRecursiveLock: NSObject

+ (id)alloc {
    log_dbg!("[NSRecursiveLock alloc]");
    let mutex_id = env.mutex_state.init_mutex(PTHREAD_MUTEX_RECURSIVE);
    let host_object = NSLockHostObject { mutex_id, name: nil };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

// NSLocking protocol implementation
- (())lock {
    log_dbg!("[(NSRecursiveLock *){:?} lock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.lock_mutex(host_object.mutex_id).unwrap();
}
- (())unlock {
    log_dbg!("[(NSRecursiveLock *){:?} unlock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if !env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        echo!("*** -[NSRecursiveLock unlock]: lock (<NSRecursiveLock: {:?}> '{:?}') unlocked when not locked", this, host_object.name);
    }
    env.unlock_mutex(host_object.mutex_id).unwrap();
}

- (bool)tryLock {
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if env.mutex_state.mutex_is_locked(host_object.mutex_id) {
        false
    } else {
        env.lock_mutex(host_object.mutex_id).is_ok()
    }
}

- (())setName:(id)name { // NSString *
    let old_name = env.objc.borrow::<NSLockHostObject>(this).name;
    // @property(copy), name has to be copied
    env.objc.borrow_mut::<NSLockHostObject>(this).name = msg![env; name copy];
    release(env, old_name);
}
- (id)name {
    env.objc.borrow::<NSLockHostObject>(this).name
}

- (())dealloc {
    log_dbg!("[(NSRecursiveLock *){:?} dealloc]", this);
    release(env, env.objc.borrow::<NSLockHostObject>(this).name);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    env.mutex_state.destroy_mutex(host_object.mutex_id).unwrap();
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation NSConditionLock: NSObject

+ (id)alloc {
    log_dbg!("[NSConditionLock alloc]");
    let mutex = env.mem.alloc(guest_size_of::<pthread_mutex_t>()).cast();
    let cond = env.mem.alloc(guest_size_of::<pthread_cond_t>()).cast();
    pthread_mutex_init(env, mutex, ConstPtr::null());
    pthread_cond_init(env, cond, ConstPtr::null());
    let host_object = NSConditionLockHostObject { mutex, cond, condition: 0, name: nil };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (id)initWithCondition:(NSInteger)condition {
    env.objc.borrow_mut::<NSConditionLockHostObject>(this).condition = condition;
    this
}

- (NSInteger)condition {
    env.objc.borrow::<NSConditionLockHostObject>(this).condition
}

// NSLocking protocol implementation
- (())lock {
    log_dbg!("[(NSConditionLock *){:?} lock]", this);
    let mutex = env.objc.borrow::<NSConditionLockHostObject>(this).mutex;
    pthread_mutex_lock(env, mutex);
}
- (())unlock {
    log_dbg!("[(NSConditionLock *){:?} unlock]", this);
    let mutex = env.objc.borrow::<NSConditionLockHostObject>(this).mutex;
    let mutex_data = env.mem.read(mutex);
    if !env.mutex_state.mutex_is_locked(mutex_data.mutex_id) {
        let name = env.objc.borrow::<NSConditionLockHostObject>(this).name;
        echo!("*** -[NSConditionLock unlock]: lock (<NSConditionLock: {:?}> '{:?}') unlocked when not locked", this, name);
    }
    pthread_mutex_unlock(env, mutex);
}

- (())lockWhenCondition:(NSInteger)condition {
    log_dbg!("[(NSConditionLock *){:?} lockWhenCondition:{}]", this, condition);
    let &NSConditionLockHostObject { mutex, cond, .. } = env.objc.borrow(this);
    pthread_mutex_lock(env, mutex);
    while env.objc.borrow::<NSConditionLockHostObject>(this).condition != condition {
        pthread_cond_wait(env, cond, mutex);
    }
}

- (bool)tryLock {
    let mutex = env.objc.borrow::<NSConditionLockHostObject>(this).mutex;
    pthread_mutex_trylock(env, mutex) == 0
}

- (bool)tryLockWhenCondition:(NSInteger)condition {
    let &NSConditionLockHostObject { mutex, condition: current, .. } = env.objc.borrow(this);
    if current != condition {
        false
    } else {
        pthread_mutex_trylock(env, mutex) == 0
    }
}

- (())unlockWithCondition:(NSInteger)condition {
    log_dbg!("[(NSConditionLock *){:?} unlockWithCondition:{}]", this, condition);
    let &NSConditionLockHostObject { mutex, cond, .. } = env.objc.borrow(this);
    env.objc.borrow_mut::<NSConditionLockHostObject>(this).condition = condition;
    pthread_cond_broadcast(env, cond);
    pthread_mutex_unlock(env, mutex);
}

- (())setName:(id)name { // NSString *
    let old_name = env.objc.borrow::<NSConditionLockHostObject>(this).name;
    // @property(copy), name has to be copied
    env.objc.borrow_mut::<NSConditionLockHostObject>(this).name = msg![env; name copy];
    release(env, old_name);
}
- (id)name {
    env.objc.borrow::<NSConditionLockHostObject>(this).name
}

- (())dealloc {
    log_dbg!("[(NSConditionLock *){:?} dealloc]", this);
    release(env, env.objc.borrow::<NSConditionLockHostObject>(this).name);
    let &NSConditionLockHostObject { mutex, cond, .. } = env.objc.borrow(this);
    pthread_cond_destroy(env, cond);
    pthread_mutex_destroy(env, mutex);
    env.mem.free(mutex.cast());
    env.mem.free(cond.cast());
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
