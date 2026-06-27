/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFSet` and `CFMutableSet`.
//!
//! These are toll-free bridged to `NSSet` / `NSMutableSet` in Apple's
//! implementation. Here they are the same type.
//!
//! References:
//! - Apple [CFSet Reference](https://developer.apple.com/documentation/corefoundation/cfset-rul)
//! - Apple [CFMutableSet Reference](https://developer.apple.com/documentation/corefoundation/cfmutableset)

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::{CFIndex, CFTypeRef};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, Ptr};
use crate::objc::{id, msg, msg_class, nil};
use crate::Environment;

pub type CFSetRef = CFTypeRef;
pub type CFMutableSetRef = CFTypeRef;

/// Apple's `CFSetCallBacks`, declared in `CFSet.h` as 4 function
/// pointers (retain, release, copyDescription, equal) plus a hash
/// function. We accept any pointer (or NULL) — `kCFTypeSetCallBacks`
/// is what almost every caller passes, and our underlying NSSet
/// already handles retain/release through Objective-C semantics.
#[repr(C, packed)]
#[allow(dead_code)]
pub struct CFSetCallBacks {
    pub version: CFIndex,
    pub retain: ConstVoidPtr,
    pub release: ConstVoidPtr,
    pub copy_description: ConstVoidPtr,
    pub equal: ConstVoidPtr,
    pub hash: ConstVoidPtr,
}
unsafe impl crate::mem::SafeRead for CFSetCallBacks {}

// MARK: - Constructors

fn CFSetCreate(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    values: ConstPtr<ConstVoidPtr>,
    num_values: CFIndex,
    callbacks: ConstPtr<CFSetCallBacks>,
) -> CFSetRef {
    let set = CFSetCreateMutable(env, allocator, num_values, callbacks);
    if !values.is_null() {
        for i in 0..num_values as u32 {
            let value: ConstVoidPtr = env.mem.read(values + i);
            let value_id: id = value.cast().cast_mut();
            () = msg![env; set addObject:value_id];
        }
    }
    set
}

fn CFSetCreateCopy(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    set: CFSetRef,
) -> CFSetRef {
    if !allocator.is_null() && allocator != kCFAllocatorDefault {
        log_dbg!("CFSetCreateCopy: ignoring custom allocator {:?}", allocator);
    }
    if set == nil {
        return nil;
    }
    let new: id = msg_class![env; NSSet alloc];
    msg![env; new initWithSet:set]
}

/// Apple docs: "Creates a new mutable set." The capacity argument is a
/// soft hint; `0` means "no upper bound". `callbacks` is normally
/// `kCFTypeSetCallBacks` (retain/release/equal of CF types) or NULL
/// (no memory management). NSMutableSet already implements those
/// semantics for any object placed into it, so we forward to it
/// regardless.
pub fn CFSetCreateMutable(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    _capacity: CFIndex,
    _callbacks: ConstPtr<CFSetCallBacks>,
) -> CFMutableSetRef {
    let new: id = msg_class![env; NSMutableSet alloc];
    msg![env; new init]
}

fn CFSetCreateMutableCopy(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    _capacity: CFIndex,
    set: CFSetRef,
) -> CFMutableSetRef {
    if !allocator.is_null() && allocator != kCFAllocatorDefault {
        log_dbg!(
            "CFSetCreateMutableCopy: ignoring custom allocator {:?}",
            allocator
        );
    }
    let new: id = msg_class![env; NSMutableSet alloc];
    if set == nil {
        msg![env; new init]
    } else {
        msg![env; new initWithSet:set]
    }
}

// MARK: - Inspectors

fn CFSetGetCount(env: &mut Environment, set: CFSetRef) -> CFIndex {
    if set == nil {
        return 0;
    }
    let count: u32 = msg![env; set count];
    count as CFIndex
}

fn CFSetContainsValue(
    env: &mut Environment,
    set: CFSetRef,
    value: ConstVoidPtr,
) -> bool {
    if set == nil {
        return false;
    }
    let value_id: id = value.cast().cast_mut();
    msg![env; set containsObject:value_id]
}

fn CFSetGetValue(
    env: &mut Environment,
    set: CFSetRef,
    value: ConstVoidPtr,
) -> ConstVoidPtr {
    if set == nil {
        return ConstPtr::null();
    }
    let value_id: id = value.cast().cast_mut();
    let member: id = msg![env; set member:value_id];
    member.cast().cast_const()
}

fn CFSetGetCountOfValue(
    env: &mut Environment,
    set: CFSetRef,
    value: ConstVoidPtr,
) -> CFIndex {
    if CFSetContainsValue(env, set, value) {
        1
    } else {
        0
    }
}

fn CFSetGetValues(
    env: &mut Environment,
    set: CFSetRef,
    values: MutPtr<ConstVoidPtr>,
) {
    if set == nil || values.is_null() {
        return;
    }
    let arr: id = msg![env; set allObjects];
    let count: u32 = msg![env; arr count];
    for i in 0..count {
        let obj: id = msg![env; arr objectAtIndex:i];
        env.mem.write(values + i, obj.cast().cast_const());
    }
}

// MARK: - Mutation

fn CFSetAddValue(
    env: &mut Environment,
    set: CFMutableSetRef,
    value: ConstVoidPtr,
) {
    if set == nil {
        return;
    }
    let value_id: id = value.cast().cast_mut();
    () = msg![env; set addObject:value_id];
}

fn CFSetReplaceValue(
    env: &mut Environment,
    set: CFMutableSetRef,
    value: ConstVoidPtr,
) {
    if set == nil {
        return;
    }
    let value_id: id = value.cast().cast_mut();
    let contains: bool = msg![env; set containsObject:value_id];
    if !contains {
        return;
    }
    () = msg![env; set removeObject:value_id];
    () = msg![env; set addObject:value_id];
}

fn CFSetSetValue(
    env: &mut Environment,
    set: CFMutableSetRef,
    value: ConstVoidPtr,
) {
    // Apple docs: "Sets a value in a mutable set. If `value` is not
    // already in the set, this function is equivalent to
    // CFSetAddValue; otherwise it replaces the existing value with the
    // new one."
    if set == nil {
        return;
    }
    let value_id: id = value.cast().cast_mut();
    let contains: bool = msg![env; set containsObject:value_id];
    if contains {
        () = msg![env; set removeObject:value_id];
    }
    () = msg![env; set addObject:value_id];
}

fn CFSetRemoveValue(
    env: &mut Environment,
    set: CFMutableSetRef,
    value: ConstVoidPtr,
) {
    if set == nil {
        return;
    }
    let value_id: id = value.cast().cast_mut();
    () = msg![env; set removeObject:value_id];
}

/// Apple docs: "Removes all values from the set." Implementation
/// forwards to `-[NSMutableSet removeAllObjects]`, which the existing
/// touchHLE NSMutableSet implements properly (releases each member
/// per the documented Cocoa semantics, which mirror
/// `kCFTypeSetCallBacks`'s release callback).
pub fn CFSetRemoveAllValues(env: &mut Environment, set: CFMutableSetRef) {
    if set == nil {
        return;
    }
    () = msg![env; set removeAllObjects];
}

/// Non-lazy export of `kCFTypeSetCallBacks`, Apple's CoreFoundation
/// canonical callbacks for `CFSet` storing CF-types
/// (retain/release/equal/hash). The real implementation lives in
/// CoreFoundation and forwards retain/release to `CFRetain`/`CFRelease`,
/// equality to `CFEqual`, and hashing to `CFHash`. Our `NSSet`-backed
/// `CFSet` doesn't actually invoke these callbacks (it uses Objective-C
/// retain/release/hash directly), but apps that read the struct (or
/// take its address) still need a valid pointer with the documented
/// `{version, retain, release, copyDescription, equal, hash}` layout.
///
/// We satisfy the contract by allocating a zero-initialised `CFSetCallBacks`
/// — every function pointer is NULL, which CoreFoundation documents
/// (`<CoreFoundation/CFSet.h>`) as "use default behaviour", and our
/// `CFSetCreateMutable` already ignores the actual callbacks anyway.
pub const CONSTANTS: ConstantExports = &[(
    "_kCFTypeSetCallBacks",
    HostConstant::Custom(|env| {
        let cb = CFSetCallBacks {
            version: 0,
            retain: Ptr::null(),
            release: Ptr::null(),
            copy_description: Ptr::null(),
            equal: Ptr::null(),
            hash: Ptr::null(),
        };
        env.mem.alloc_and_write(cb).cast_void().cast_const()
    }),
)];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFSetCreate(_, _, _, _)),
    export_c_func!(CFSetCreateCopy(_, _)),
    export_c_func!(CFSetCreateMutable(_, _, _)),
    export_c_func!(CFSetCreateMutableCopy(_, _, _)),
    export_c_func!(CFSetGetCount(_)),
    export_c_func!(CFSetGetCountOfValue(_, _)),
    export_c_func!(CFSetContainsValue(_, _)),
    export_c_func!(CFSetGetValue(_, _)),
    export_c_func!(CFSetGetValues(_, _)),
    export_c_func!(CFSetAddValue(_, _)),
    export_c_func!(CFSetReplaceValue(_, _)),
    export_c_func!(CFSetSetValue(_, _)),
    export_c_func!(CFSetRemoveValue(_, _)),
    export_c_func!(CFSetRemoveAllValues(_)),
];
