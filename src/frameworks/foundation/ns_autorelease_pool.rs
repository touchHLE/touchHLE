/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSAutoreleasePool`.

use crate::objc::{id, msg, objc_classes, release, ClassExports, HostObject, NSZonePtr};
use crate::{Environment, ThreadId};
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    pool_stacks: HashMap<ThreadId, Vec<id>>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.foundation.ns_autorelease_pool
    }
}

struct NSAutoreleasePoolHostObject {
    original_thread: ThreadId,
    /// This is allowed to contain duplicates, which get released several times!
    objects: Vec<id>,
}
impl HostObject for NSAutoreleasePoolHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSAutoreleasePool: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSAutoreleasePoolHostObject {
        original_thread: env.current_thread,
        objects: Vec::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (())addObject:(id)obj {
    let current_thread = env.current_thread;
    let pool_stack = State::get(env).pool_stacks.get(&current_thread).unwrap();
    if let Some(current_pool) = pool_stack.last().copied() {
        msg![env; current_pool addObject:obj]
    } else {
        log_dbg!("Warning: no active NSAutoreleasePool, leaking {:?}, current thread {}", obj, current_thread);
    }
}

- (id)init {
    let current_thread = env.current_thread;
    let pool_stack = State::get(env).pool_stacks
        .entry(current_thread)
        .or_default();
    pool_stack.push(this);
    log_dbg!("New pool: {:?}, current thread {}", this, current_thread);
    this
}

- (())addObject:(id)obj {
    env.objc.borrow_mut::<NSAutoreleasePoolHostObject>(this).objects.push(obj);
}

- (id)retain {
    // TODO: throw proper exception?
    panic!("NSAutoreleasePool can't be retained!");
}
- (id)autorelease {
    // TODO: throw proper exception?
    panic!("NSAutoreleasePool can't be autoreleased!");
}

- (())drain {
    msg![env; this release]
}

- (())dealloc {
    let current_thread = env.current_thread;
    log_dbg!("Draining pool: {:?}, current thread {}", this, current_thread);
    let host_obj: &mut NSAutoreleasePoolHostObject = env.objc.borrow_mut(this);
    // It's unclear what should happen when draining a pool on the wrong thread,
    // but we prefer to be conservative here
    assert_eq!(host_obj.original_thread, current_thread);
    let pool_stack = &mut env
        .framework_state
        .foundation
        .ns_autorelease_pool
        .pool_stacks
        .get_mut(&current_thread).unwrap();
    let pop_res = pool_stack.pop();
    assert!(pop_res == Some(this));
    let objects = std::mem::take(&mut host_obj.objects);
    env.objc.dealloc_object(this, &mut env.mem);
    for object in objects {
        release(env, object);
    }
}

@end

};
