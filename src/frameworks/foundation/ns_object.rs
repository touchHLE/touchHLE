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
use super::{NSTimeInterval, NSUInteger};
use crate::frameworks::foundation::ns_thread::detach_new_thread_inner;
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, msg_send_no_type_checking, nil, objc_classes,
    retain, Class, ClassExports, NSZonePtr, ObjC, TrivialHostObject, SEL,
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
+ (bool)isSubclassOfClass:(Class)class {
    env.objc.class_is_subclass_of(this, class)
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

// Возвращаем u32 (адрес), так как IMP не реализует GuestRet
+ (u32)instanceMethodForSelector:(SEL)_selector {
    log!("Warning: instanceMethodForSelector: for {:?} is stubbed", _selector);
    0
}

+ (bool)accessInstanceVariablesDirectly {
    true
}

+ (id)description {
    let name = env.objc.get_class_name(this);
    let str = from_rust_string(env, name.to_string());
    autorelease(env, str)
}

+ (id)debugDescription {
    msg![env; this description]
}

- (id)init {
    this
}

- (NSUInteger)retainCount {
    env.objc.get_refcount(this).into()
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

// To not confuse with isEqualTo:, which is
// a category of NSWhoseSpecifier!
// Reference https://nshipster.com/equality
- (bool)isEqual:(id)other {
    this == other
}

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
    let key_string = to_rust_string(env, key); // TODO: avoid copy?
    assert!(key_string.is_ascii()); // TODO: do we have to handle non-ASCII keys?
    let camel_case_key_string = format!("{}{}", key_string.as_bytes()[0].to_ascii_uppercase() as char, &key_string[1..]);

    let class = msg![env; this class];

    assert!(value != nil);

    let value_class = msg![env; value class];
    let ns_value_class = env.objc.get_known_class("NSValue", &mut env.mem);
    assert!(!env.objc.class_is_subclass_of(value_class, ns_value_class));

    if let Some(sel) = env.objc.lookup_selector(&format!("set{camel_case_key_string}:")) {
        if env.objc.class_has_method(class, sel) {
            () = msg_send(env, (this, sel, value));
            return;
        }
    }

    if let Some(sel) = env.objc.lookup_selector(&format!("_set{camel_case_key_string}:")) {
        if env.objc.class_has_method(class, sel) {
            () = msg_send(env, (this, sel, value));
            return;
        }
    }

    let sel = env.objc.lookup_selector("accessInstanceVariablesDirectly").unwrap();
    let accessInstanceVariablesDirectly = msg_send(env, (class, sel));
    if accessInstanceVariablesDirectly {
        if let Some(ivar_ptr) = env.objc.object_lookup_ivar(&env.mem, this, &format!("_{key_string}"))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("_is{camel_case_key_string}")))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("{key_string}")))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("is{camel_case_key_string}"))
        ) {
            retain(env, value);
            env.mem.write(ivar_ptr.cast(), value);
            return;
        }
    }

    let sel = env.objc.lookup_selector("setValue:forUndefinedKey:").unwrap();
    () = msg_send(env, (this, sel, value, key));
}

- (())setValue:(id)_value
forUndefinedKey:(id)key { // NSString*
    let class: Class = ObjC::read_isa(this, &env.mem);
    let class_name_string = env.objc.get_class_name(class).to_owned(); // TODO: Avoid copying
    let key_string = to_rust_string(env, key);
    panic!("Object {:?} of class {:?} ({:?}) does not have a setter for {} ({:?})\
        \nAvailable selectors: {}\nAvailable ivars: {}",
        this, class_name_string, class, key_string, key,
        env.objc.debug_all_class_selectors_as_strings(&env.mem, class).join(", "),
        env.objc.debug_all_class_ivars_as_strings(class).join(", "));
}

- (bool)respondsToSelector:(SEL)selector {
    env.objc.object_has_method(&env.mem, this, selector)
}

// Возвращаем u32 (адрес), так как IMP не реализует GuestRet
- (u32)methodForSelector:(SEL)_selector {
    log!("Warning: methodForSelector: for {:?} is stubbed", _selector);
    0
}

- (id)performSelector:(SEL)sel {
    assert!(!sel.is_null());
    msg_send_no_type_checking(env, (this, sel))
}

- (id)performSelector:(SEL)sel
           withObject:(id)o1 {
    assert!(!sel.is_null());
    msg_send_no_type_checking(env, (this, sel, o1))
}

- (id)performSelector:(SEL)sel
           withObject:(id)o1
           withObject:(id)o2 {
    assert!(!sel.is_null());
    msg_send_no_type_checking(env, (this, sel, o1, o2))
}

- (())performSelectorInBackground:(SEL)sel
                       withObject:(id)arg {
    detach_new_thread_inner(env, sel, this, arg, /* tolerate_type_mismatch: */ true)
}

- (())performSelector:(SEL)sel withObject:(id)arg afterDelay:(NSTimeInterval)delay {
    log_dbg!("performSelector:{} withObject:{:?} afterDelay:{}", sel.as_str(&env.mem), arg, delay);

    let sel_key: id = get_static_str(env, "SEL");
    let sel_str = from_rust_string(env, sel.as_str(&env.mem).to_string());
    let arg_key: id = get_static_str(env, "arg");
    let dict = dict_from_keys_and_objects(env, &[(sel_key, sel_str), (arg_key, arg)]);

    let selector = env.objc.lookup_selector("_touchHLE_timerFireMethod:").unwrap();
    let timer:id = msg_class![env; NSTimer timerWithTimeInterval:delay
                                              target:this
                                            selector:selector
                                            userInfo:dict
                                             repeats:false];

    let run_loop: id = msg_class![env; NSRunLoop mainRunLoop];
    let mode: id = get_static_str(env, NSDefaultRunLoopMode);
    () = msg![env; run_loop addTimer:timer forMode:mode];
}

- (())performSelectorOnMainThread:(SEL)sel withObject:(id)arg waitUntilDone:(bool)wait {
    log_dbg!("performSelectorOnMainThread:{} withObject:{:?} waitUntilDone:{}", sel.as_str(&env.mem), arg, wait);
    if wait && env.current_thread == 0 {
        if sel.as_str(&env.mem).ends_with(':') {
            () = msg_send(env, (this, sel, arg));
        } else {
            assert!(arg.is_null());
            () = msg_send(env, (this, sel));
        }
        return;
    }
    
    // ... (Hacks for specific games)
    
    assert!(!wait);
    msg![env; this performSelector:sel withObject:arg afterDelay:0.0]
}

- (())_touchHLE_timerFireMethod:(id)which { // NSTimer *
    let dict: id = msg![env; which userInfo];

    let sel_key: id = get_static_str(env, "SEL");
    let sel_str_id: id = msg![env; dict objectForKey:sel_key];
    let sel_str = to_rust_string(env, sel_str_id);
    let sel = env.objc.lookup_selector(&sel_str).unwrap();

    let arg_key: id = get_static_str(env, "arg");
    let arg: id = msg![env; dict objectForKey:arg_key];

    if sel.as_str(&env.mem).ends_with(':') {
        () = msg_send(env, (this, sel, arg));
    } else {
        () = msg_send(env, (this, sel));
    }
}

- (())awakeFromNib {
    // no-op
}

@end

};
