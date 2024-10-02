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
use crate::mem::MutVoidPtr;
use crate::objc::{
    id, msg, msg_class, msg_send, nil, objc_classes, retain, Class, ClassExports, NSZonePtr, ObjC,
    TrivialHostObject, SEL, TYPE_BOOL, TYPE_CHAR, TYPE_CHAR_PTR, TYPE_CLASS, TYPE_DOUBLE,
    TYPE_FLOAT, TYPE_ID, TYPE_INT, TYPE_LONG, TYPE_LONGLONG, TYPE_SEL, TYPE_SHORT, TYPE_UCHAR,
    TYPE_UINT, TYPE_ULONG, TYPE_ULONGLONG, TYPE_UNDEF, TYPE_USHORT, TYPE_VOID,
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

+ (bool)accessInstanceVariablesDirectly {
    true
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
// https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/KeyValueCoding/SearchImplementation.html
- (())setValue:(id)value
       forKey:(id)key { // NSString*
    let key_string = to_rust_string(env, key); // TODO: avoid copy?
    assert!(key_string.is_ascii()); // TODO: do we have to handle non-ASCII keys?
    let camel_case_key_string = format!("{}{}", key_string.as_bytes()[0].to_ascii_uppercase() as char, &key_string[1..]);

    let class = msg![env; this class];

    // TODO: If value is nil, the target ivar/method argument type must be
    // checked. If it's non-object type, invoke setNilValueForKey:
    assert!(value != nil);

    let value_class = msg![env; value class];
    let ns_value_class = env.objc.get_known_class("NSValue", &mut env.mem);

    // Look for the first accessor named set<Key>: or _set<Key>, in that order.
    // If found, invoke it with the input value (or unwrapped value, as needed)
    // and finish.
    if let Some(sel) = env.objc.lookup_selector(&format!("set{}:", camel_case_key_string)).filter(|&sel| env.objc.class_has_method(class, sel))
        .or_else(|| env.objc.lookup_selector(&format!("_set{}:", camel_case_key_string))).filter(|&sel| env.objc.class_has_method(class, sel)
    ) {
        if env.objc.class_is_subclass_of(value_class, ns_value_class) {
            // If value is a NSValue, it must be unwrapped
            // Find the selector's first (and only) argument type:
            let types = env.objc.get_class_method(class, sel).types.clone();
            let mut type_index = 0;
            let mut argument_type = None;
            assert_ne!(types.len(), 0);
            for character in types.chars() {
                // TODO: Handle arrays, structs, unions, bitfields, pointers
                // and other types that span multiple chars
                if matches!(character, TYPE_ID | TYPE_CLASS | TYPE_SEL | TYPE_CHAR | TYPE_UCHAR | TYPE_SHORT | TYPE_USHORT | TYPE_INT | TYPE_UINT | TYPE_LONG | TYPE_ULONG | TYPE_LONGLONG | TYPE_ULONGLONG | TYPE_FLOAT | TYPE_DOUBLE | TYPE_BOOL | TYPE_VOID | TYPE_CHAR_PTR | TYPE_UNDEF) {
                    match type_index {
                        // First type is the return type
                        0 => assert_eq!(character, TYPE_VOID),
                        // Second type in methods must be the selector
                        1 => assert_eq!(character, TYPE_SEL),
                        // Third type is the method's first argument type
                        2 => argument_type = Some(character),
                        // Panic if there's more than one type
                        _ => panic!(),
                    }
                    type_index += 1;
                }
            }

            match argument_type.unwrap() {
                TYPE_BOOL => {
                    let value: bool = msg![env; value boolValue];
                    () = msg_send(env, (this, sel, value));
                },
                TYPE_DOUBLE => {
                    let value: f64 = msg![env; value doubleValue];
                    () = msg_send(env, (this, sel, value));
                },
                TYPE_FLOAT => {
                    let value: f32 = msg![env; value floatValue];
                    () = msg_send(env, (this, sel, value));
                },
                TYPE_INT => {
                    let value: i32 = msg![env; value intValue];
                    () = msg_send(env, (this, sel, value));
                },
                TYPE_LONGLONG => {
                    let value: i64 = msg![env; value longLongValue];
                    () = msg_send(env, (this, sel, value));
                },
                TYPE_UINT => {
                    let value: u32 = msg![env; value unsignedIntValue];
                    () = msg_send(env, (this, sel, value));
                },
                TYPE_ULONGLONG => {
                    let value: u64 = msg![env; value unsignedLongLongValue];
                    () = msg_send(env, (this, sel, value));
                },
                argument_type => unimplemented!("TODO: [(NSObject*){:?} setValue:{:?} forKey:{:?}]: Received an NSValue which must be unwrapped to type {}", this, value, key, argument_type)
            };
        } else {
            () = msg_send(env, (this, sel, value));
        }
        return;
    }

    // If no simple accessor is found, and if the class method
    // accessInstanceVariablesDirectly returns YES, look for an instance
    // variable with a name like _<key>, _is<Key>, <key>, or is<Key>,
    // in that order.
    // If found, set the variable directly with the input value
    // (or unwrapped value) and finish.
    let sel = env.objc.lookup_selector("accessInstanceVariablesDirectly").unwrap();
    let accessInstanceVariablesDirectly = msg_send(env, (class, sel));
    if accessInstanceVariablesDirectly {
        if let Some((ivar_ptr, ivar_type)) = env.objc.object_lookup_ivar(&env.mem, this, &format!("_{}", key_string))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("_is{}", camel_case_key_string)))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("{}", key_string)))
            .or_else(|| env.objc.object_lookup_ivar(&env.mem, this, &format!("is{}", camel_case_key_string))
        ) {
            if env.objc.class_is_subclass_of(value_class, ns_value_class) {
                // If value is a NSValue, it must be unwrapped
                match ivar_type.chars().next().unwrap() {
                    TYPE_BOOL => {
                        let value: bool = msg![env; value boolValue];
                        env.mem.write(ivar_ptr.cast(), value);
                    },
                    TYPE_DOUBLE => {
                        let value: f64 = msg![env; value doubleValue];
                        env.mem.write(ivar_ptr.cast(), value);
                    },
                    TYPE_FLOAT => {
                        let value: f32 = msg![env; value floatValue];
                        env.mem.write(ivar_ptr.cast(), value);
                    },
                    TYPE_INT => {
                        let value: i32 = msg![env; value intValue];
                        env.mem.write(ivar_ptr.cast(), value);
                    },
                    TYPE_LONGLONG => {
                        let value: i64 = msg![env; value longLongValue];
                        env.mem.write(ivar_ptr.cast(), value);
                    },
                    TYPE_UINT => {
                        let value: u32 = msg![env; value unsignedIntValue];
                        env.mem.write(ivar_ptr.cast(), value);
                    },
                    TYPE_ULONGLONG => {
                        let value: u64 = msg![env; value unsignedLongLongValue];
                        env.mem.write(ivar_ptr.cast(), value);
                    },
                    _ => unimplemented!("TODO: [(NSObject*){:?} setValue:{:?} forKey:{:?}]: Received an NSValue which must be unwrapped to type {}", this, value, key, ivar_type)
                };
            } else {
                retain(env, value);
                env.mem.write(ivar_ptr.cast(), value);
            };
            return;
        }
    }

    // Upon finding no accessor or instance variable,
    // invoke setValue:forUndefinedKey:.
    // This raises an exception by default, but a subclass of NSObject
    // may provide key-specific behavior.
    let sel = env.objc.lookup_selector("setValue:forUndefinedKey:").unwrap();
    () = msg_send(env, (this, sel, value, key));
}

- (())setValue:(id)_value
forUndefinedKey:(id)key { // NSString*
    // TODO: Raise NSUnknownKeyException
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

- (())performSelector:(SEL)sel withObject:(id)arg afterDelay:(NSTimeInterval)delay {
    log_dbg!("performSelector:{} withObject:{:?} afterDelay:{}", sel.as_str(&env.mem), arg, delay);

    let sel_key: id = get_static_str(env, "SEL");
    let sel_str = from_rust_string(env, sel.as_str(&env.mem).to_string());
    let arg_key: id = get_static_str(env, "arg");
    let dict = dict_from_keys_and_objects(env, &[(sel_key, sel_str), (arg_key, arg)]);

    // TODO: using timer is not the most efficient implementation, but does work
    // Proper implementation requires a message queue in the run loop
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
        () = msg_send(env, (this, sel, arg));
        return;
    }
    if env.bundle.bundle_identifier().starts_with("com.gameloft.POP") && sel == env.objc.lookup_selector("startMovie:").unwrap() && wait {
        log!("Applying game-specific hack for PoP: WW: ignoring performSelectorOnMainThread:SEL(startMovie:) waitUntilDone:true");
        return;
    }
    if env.bundle.bundle_identifier().starts_with("com.gameloft.Asphalt5") && (sel == env.objc.lookup_selector("startMovie:").unwrap() || sel == env.objc.lookup_selector("stopMovie:").unwrap()) && wait {
        log!("Applying game-specific hack for Asphalt5: ignoring performSelectorOnMainThread:SEL({}) waitUntilDone:true", sel.as_str(&env.mem));
        return;
    }
    // TODO: support waiting
    // This would require tail calls for message send or a switch to async model
    assert!(!wait);

    // The current implementation of performSelector:withObject:afterDelay
    // already runs on the main thread.
    msg![env; this performSelector:sel withObject:arg afterDelay:0.0]
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
}

@end

};
