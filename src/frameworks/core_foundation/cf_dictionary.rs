/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFDictionary` and `CFMutableDictionary`.
//!
//! These are toll-free bridged to `NSDictionary` and `NSMutableDictionary` in
//! Apple's implementation. Here they are the same types.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::CFIndex;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstPtr, ConstVoidPtr, MutVoidPtr};
use crate::objc::{id, msg, msg_class, nil};
use crate::Environment;

pub type CFDictionaryRef = super::CFTypeRef;
pub type CFMutableDictionaryRef = super::CFTypeRef;

fn CFDictionaryCreateMutable(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    capacity: CFIndex,
    key_callbacks: ConstVoidPtr, // TODO, should be `const CFDictionaryKeyCallBacks*`
    value_callbacks: ConstVoidPtr, // TODO, should be `const CFDictionaryValueCallBacks*`
) -> CFMutableDictionaryRef {
    assert_eq!(allocator, kCFAllocatorDefault); // unimplemented
    assert_eq!(capacity, 0); // TODO: fixed capacity support
    assert!(key_callbacks.is_null()); // TODO: support retaining etc
    assert!(value_callbacks.is_null()); // TODO: support retaining etc

    msg_class![env; _touchHLE_NSMutableDictionary_non_retaining new]
}

fn CFDictionaryAddValue(
    env: &mut Environment,
    dict: CFMutableDictionaryRef,
    key: ConstVoidPtr,
    value: ConstVoidPtr,
) {
    let key: id = key.cast().cast_mut();
    let res: id = msg![env; dict objectForKey:key];
    log_dbg!(
        "CFDictionaryAddValue dict {:?} k {:?} v {:?}; res {:?}",
        dict,
        key,
        value,
        res
    );
    if res == nil {
        let value: id = value.cast().cast_mut();
        msg![env; dict setObject:value forKey:key]
    }
}

fn CFDictionarySetValue(
    env: &mut Environment,
    dict: CFMutableDictionaryRef,
    key: ConstVoidPtr,
    value: ConstVoidPtr,
) {
    log_dbg!("CFDictionarySetValue k {:?} v {:?}", key, value);
    let key: id = key.cast().cast_mut();
    let value: id = value.cast().cast_mut();
    msg![env; dict setObject:value forKey:key]
}

fn CFDictionaryRemoveValue(env: &mut Environment, dict: CFMutableDictionaryRef, key: ConstVoidPtr) {
    let key: id = key.cast().cast_mut();
    log_dbg!("CFDictionaryRemoveValue dict {:?} key {:?}", dict, key);
    () = msg![env; dict removeObjectForKey:key];
}

fn CFDictionaryRemoveAllValues(env: &mut Environment, dict: CFMutableDictionaryRef) {
    // TODO: use keyEnumerator
    let keys_arr: id = msg![env; dict allKeys];
    let enumerator: id = msg![env; keys_arr objectEnumerator];
    let mut key: id;
    loop {
        key = msg![env; enumerator nextObject];
        if key == nil {
            break;
        }
        CFDictionaryRemoveValue(env, dict, key.cast().cast_const());
    }
}

fn CFDictionaryGetValue(
    env: &mut Environment,
    dict: CFMutableDictionaryRef,
    key: ConstVoidPtr,
) -> ConstVoidPtr {
    let key: id = key.cast().cast_mut();
    let res: id = msg![env; dict objectForKey:key];
    res.cast().cast_const()
}

fn CFDictionaryGetCount(env: &mut Environment, dict: CFDictionaryRef) -> CFIndex {
    let count: NSUInteger = msg![env; dict count];
    log_dbg!("CFDictionaryGetCount dict {:?} {}", dict, count);
    count.try_into().unwrap()
}

fn CFDictionaryGetKeysAndValues(
    env: &mut Environment,
    dict: CFDictionaryRef,
    keys: ConstPtr<MutVoidPtr>,
    values: ConstPtr<MutVoidPtr>,
) {
    let mut key_ptr = keys.cast_mut();
    let mut val_ptr = values.cast_mut();
    // TODO: use keyEnumerator
    let keys_arr: id = msg![env; dict allKeys];
    let enumerator: id = msg![env; keys_arr objectEnumerator];
    let mut key: id;
    let mut val: id;
    loop {
        key = msg![env; enumerator nextObject];
        if key == nil {
            break;
        }
        if !key_ptr.is_null() {
            env.mem.write(key_ptr, key.cast());
            key_ptr += 1;
        }
        if !val_ptr.is_null() {
            val = msg![env; dict objectForKey:key];
            log_dbg!(
                "CFDictionaryGetKeysAndValues dict {:?} key {:?} val {:?}",
                dict,
                key,
                val
            );
            env.mem.write(val_ptr, val.cast());
            val_ptr += 1;
        }
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFDictionaryCreateMutable(_, _, _, _)),
    export_c_func!(CFDictionaryAddValue(_, _, _)),
    export_c_func!(CFDictionarySetValue(_, _, _)),
    export_c_func!(CFDictionaryRemoveValue(_, _)),
    export_c_func!(CFDictionaryRemoveAllValues(_)),
    export_c_func!(CFDictionaryGetValue(_, _)),
    export_c_func!(CFDictionaryGetCount(_)),
    export_c_func!(CFDictionaryGetKeysAndValues(_, _, _)),
];
