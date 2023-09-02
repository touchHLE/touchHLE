/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSLock`.

use crate::environment::ThreadId;
use crate::libc::pthread::mutex::{
    pthread_mutex_destroy, pthread_mutex_init, pthread_mutex_lock, pthread_mutex_t,
    pthread_mutex_unlock,
};
use crate::mem::{guest_size_of, MutPtr};
use crate::msg;
use crate::objc::{id, nil, objc_classes, ClassExports, HostObject};

struct NSLockHostObject {
    pthread_mutex_ptr: MutPtr<pthread_mutex_t>,
    name: id,
    locked_by: Option<ThreadId>,
}
impl HostObject for NSLockHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSLock: NSObject

+ (id)alloc {
    log_dbg!("[NSLock alloc]");
    let pthread_mutex_ptr = env.mem.alloc(guest_size_of::<pthread_mutex_t>()).cast();
    assert!(pthread_mutex_init(env, pthread_mutex_ptr, nil.cast().cast_const()) == 0);
    let host_object = NSLockHostObject { pthread_mutex_ptr, name: nil, locked_by: None };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (())lock {
    log_dbg!("[(NSLock*){:?} lock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    assert!(host_object.locked_by.is_none());
    assert!(pthread_mutex_lock(env, host_object.pthread_mutex_ptr) == 0);
    env.objc.borrow_mut::<NSLockHostObject>(this).locked_by = Some(env.current_thread);
}

- (())unlock {
    log_dbg!("[(NSLock*){:?} unlock]", this);
    let host_object = env.objc.borrow::<NSLockHostObject>(this);
    if let Some(locked_by_thread) = host_object.locked_by {
        assert!(locked_by_thread == env.current_thread);
    } else {
        echo!("*** -[NSLock unlock]: lock (<NSLock: {:?}> '{:?}') unlocked when not locked", this, host_object.name);
    }
    assert!(pthread_mutex_unlock(env, host_object.pthread_mutex_ptr) == 0);
    env.objc.borrow_mut::<NSLockHostObject>(this).locked_by = None
}

- (())setName:(id)name { // NSString *
    // @property(copy), name has to be copied
    env.objc.borrow_mut::<NSLockHostObject>(this).name = msg![env; name copy];
}
- (id)name {
    env.objc.borrow::<NSLockHostObject>(this).name
}

- (())dealloc {
    log_dbg!("[(NSLock*){:?} dealloc]", this);
    let pthread_mutex_ptr = env.objc.borrow::<NSLockHostObject>(this).pthread_mutex_ptr;
    assert!(pthread_mutex_destroy(env, pthread_mutex_ptr) == 0);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
