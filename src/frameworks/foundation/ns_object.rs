/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSObject`, the root of most class hierarchies in Objective-C.
//!
//! Resources:
//! - Apple's [Advanced Memory Management Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MemoryMgmt/Articles/MemoryMgmt.html)
//!   explains how reference counting works. Note that we are interested in what
//!   it calls "manual retain-release", not ARC.
//! - Apple's [Key-Value Coding Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/KeyValueCoding/SearchImplementation.html)
//!   explains the algorithm `setValue:forKey:` should follow.
//!
//! See also: [crate::objc], especially the `objects` module.

use super::ns_dictionary::dict_from_keys_and_objects;
use super::ns_run_loop::NSDefaultRunLoopMode;
use super::ns_string::{from_rust_string, get_static_str, to_rust_string};
use super::{NSInteger, NSUInteger};
use crate::environment::ThreadId;
use crate::libc::posix_io::{O_CREAT, O_EXCL};
use crate::libc::semaphore::{sem_close, sem_open, sem_post, sem_t, sem_unlink, sem_wait};
use crate::mem::{MutPtr, MutVoidPtr};
use crate::objc::{
    id, msg, msg_class, msg_send, nil, objc_classes, release, retain, Class, ClassExports,
    HostObject, NSZonePtr, ObjC, TrivialHostObject, SEL,
};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSObject

+ (id)alloc {
    msg![env; this allocWithZone:(MutVoidPtr::null())]
}
+ (id)allocWithZone:(NSZonePtr)_zone { // struct _NSZone*
    log_dbg!("[{:?} allocWithZone:]", this);
    env.objc.alloc_object(this, Box::new(TrivialHostObject), &mut env.mem)
}

+ (id)new {
    let new_object: id = msg![env; this alloc];
    msg![env; new_object init]
}

+ (Class)class {
    this
}

// See the instance method section for the normal versions of these.
+ (id)retain {
    this // classes are not refcounted
}
+ (())release {
    // classes are not refcounted
}
+ (())autorelease {
    // classes are not refcounted
}

+ (bool)instancesRespondToSelector:(SEL)selector {
    env.objc.class_has_method(this, selector)
}

- (id)init {
    this
}


- (id)retain {
    log_dbg!("[{:?} retain]", this);
    env.objc.increment_refcount(this);
    this
}
- (())release {
    log_dbg!("[{:?} release]", this);
    if env.objc.decrement_refcount(this) {
        () = msg![env; this dealloc];
    }
}
- (id)autorelease {
    () = msg_class![env; NSAutoreleasePool addObject:this];
    this
}

- (())dealloc {
    log_dbg!("[{:?} dealloc]", this);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (Class)class {
    ObjC::read_isa(this, &env.mem)
}
- (bool)isMemberOfClass:(Class)class {
    let this_class: Class = msg![env; this class];
    class == this_class
}
- (bool)isKindOfClass:(Class)class {
    let this_class: Class = msg![env; this class];
    env.objc.class_is_subclass_of(this_class, class)
}

- (NSUInteger)hash {
    this.to_bits()
}
- (bool)isEqual:(id)other {
    this == other
}

// TODO: description and debugDescription (both the instance and class method).
// This is not hard to add, but before adding a fallback implementation of it,
// we should make sure all the Foundation classes' overrides of it are there,
// to prevent weird behavior.
// TODO: localized description methods also? (not sure if NSObject has them)

// Helper for NSCopying
- (id)copy {
    msg![env; this copyWithZone:(MutVoidPtr::null())]
}

// Helper for NSMutableCopying
- (id)mutableCopy {
    msg![env; this mutableCopyWithZone:(MutVoidPtr::null())]
}

// NSKeyValueCoding
- (())setValue:(id)value
       forKey:(id)key { // NSString*
    let key = to_rust_string(env, key); // TODO: avoid copy?
    assert!(key.is_ascii()); // TODO: do we have to handle non-ASCII keys?

    let class = msg![env; this class];

    if let Some(sel) = env.objc.lookup_selector(&format!(
        "set{}{}:",
        key.as_bytes()[0].to_ascii_uppercase() as char,
        &key[1..],
    )) {
        if env.objc.class_has_method(class, sel) {
            return msg_send(env, (this, sel, value));
        }
    }

    if let Some(sel) = env.objc.lookup_selector(&format!(
        "_set{}{}:",
        key.as_bytes()[0].to_ascii_uppercase() as char,
        &key[1..],
    )) {
        if env.objc.class_has_method(class, sel) {
            return msg_send(env, (this, sel, value));
        }
    }

    unimplemented!("TODO: object {:?} does not have simple setter method for {}, use fallback", this, key);
}

- (bool)respondsToSelector:(SEL)selector {
    let class = msg![env; this class];
    env.objc.class_has_method(class, selector)
}

- (id)performSelector:(SEL)sel {
    assert!(!sel.is_null());
    msg_send(env, (this, sel))
}

- (id)performSelector:(SEL)sel
           withObject:(id)o1 {
    assert!(!sel.is_null());
    msg_send(env, (this, sel, o1))
}

- (id)performSelector:(SEL)sel
           withObject:(id)o1
           withObject:(id)o2 {
    assert!(!sel.is_null());
    msg_send(env, (this, sel, o1, o2))
}

- (())performSelectorOnMainThread:(SEL)sel withObject:(id)arg waitUntilDone:(bool)wait {
    log_dbg!("performSelectorOnMainThread:{} withObject:{:?} waitUntilDone:{}", sel.as_str(&env.mem), arg, wait);
    if wait && env.current_thread == 0 {
        () = msg_send(env, (this, sel, arg));
        return;
    }
    let sem_obj = if wait {
        let sem: id = msg_class![env; _touchHLE_NSObjectSemaphore alloc];
        msg![env; sem initWithObject:this]
    } else {
        nil
    };

    let sel_key: id = get_static_str(env, "SEL");
    let sel_str = from_rust_string(env, sel.as_str(&env.mem).to_string());
    let arg_key: id = get_static_str(env, "arg");
    let sem_key: id = get_static_str(env, "sem");
    let dict = dict_from_keys_and_objects(env, &[(sel_key, sel_str), (arg_key, arg), (sem_key, sem_obj)]);

    // TODO: using timer is not the most efficient implementation,
    // but does work
    let selector = env.objc.lookup_selector("_touchHLE_timerFireMethod:").unwrap();
    let timer:id = msg_class![env; NSTimer timerWithTimeInterval:0.0
                                              target:this
                                            selector:selector
                                            userInfo:dict
                                             repeats:false];

    let run_loop: id = msg_class![env; NSRunLoop mainRunLoop];
    let mode: id = get_static_str(env, NSDefaultRunLoopMode);
    () = msg![env; run_loop addTimer:timer forMode:mode];

    let _: NSInteger = msg![env; sem_obj wait];
}

// Private method, used by performSelectorOnMainThread:withObject:waitUntilDone:
- (())_touchHLE_timerFireMethod:(id)which { // NSTimer *
    let dict: id = msg![env; which userInfo];

    let sel_key: id = get_static_str(env, "SEL");
    let sel_str_id: id = msg![env; dict objectForKey:sel_key];
    let sel_str = to_rust_string(env, sel_str_id);
    let sel = env.objc.lookup_selector(&sel_str).unwrap();

    let arg_key: id = get_static_str(env, "arg");
    let arg: id = msg![env; dict objectForKey:arg_key];

    () = msg_send(env, (this, sel, arg));

    let sem_key: id = get_static_str(env, "sem");
    let sem_obj: id = msg![env; dict objectForKey:sem_key];
    let _: NSInteger = msg![env; sem_obj post];
}

@end

@implementation _touchHLE_NSObjectSemaphore: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSObjectSemaphoreHostObject {
        orig_thread: 0,
        object: nil,
        sem: MutPtr::null()
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithObject:(id)object {
    retain(env, object);

    let name = env.mem.alloc_and_write_cstr(format!("_touchHLE_sem_{}_{:?}", env.current_thread, this).as_bytes());
    let sem = sem_open(env, name.cast_const(), O_EXCL | O_CREAT, 0, 0);
    env.mem.free(name.cast());

    let host_object: &mut NSObjectSemaphoreHostObject = env.objc.borrow_mut(this);
    host_object.orig_thread = env.current_thread;
    host_object.object = object;
    host_object.sem = sem;

    this
}

- (NSInteger)wait {
    let sem = env.objc.borrow::<NSObjectSemaphoreHostObject>(this).sem;
    let res = sem_wait(env, sem);
    assert_eq!(res, 0);
    res
}

- (NSInteger)post {
    let sem = env.objc.borrow::<NSObjectSemaphoreHostObject>(this).sem;
    let res = sem_post(env, sem);
    assert_eq!(res, 0);
    res
}

- (())dealloc {
    let &NSObjectSemaphoreHostObject {
        orig_thread,
        object,
        sem
    } = env.objc.borrow(this);

    let name = env.mem.alloc_and_write_cstr(format!("_touchHLE_sem_{}_{:?}", orig_thread, object).as_bytes());
    let res = sem_unlink(env, name.cast_const());
    assert_eq!(res, 0);
    env.mem.free(name.cast());

    let res = sem_close(env, sem);
    assert_eq!(res, 0);

    release(env, object);

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

/// Belongs to _touchHLE_NSObjectSemaphore
struct NSObjectSemaphoreHostObject {
    orig_thread: ThreadId,
    object: id,
    sem: MutPtr<sem_t>,
}
impl HostObject for NSObjectSemaphoreHostObject {}
