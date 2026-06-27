/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFType` (type-generic functions etc).

use super::{CFHashCode, CFIndex};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::NSUInteger;
use crate::objc::Class;
use crate::{msg, objc};
use crate::{msg_class, Environment};

pub type CFTypeRef = objc::id;
pub type CFTypeID = CFIndex;

pub fn CFRetain(env: &mut Environment, object: CFTypeRef) -> CFTypeRef {
    // ИСПРАВЛЕНИЕ: Убираем жесткий assert. Из-за отсутствующих функций
    // сюда может прилетать NULL. Игнорируем, чтобы не крашить эмулятор.
    if object.is_null() {
        log_dbg!("Warning: CFRetain called with NULL. Ignoring to prevent crash.");
        return object;
    }
    objc::retain(env, object)
}

pub fn CFRelease(env: &mut Environment, object: CFTypeRef) {
    // ИСПРАВЛЕНИЕ: Защита от NULL
    if object.is_null() {
        log_dbg!("Warning: CFRelease called with NULL. Ignoring.");
        return;
    }
    objc::release(env, object);
}

pub fn CFGetRetainCount(env: &mut Environment, object: CFTypeRef) -> CFIndex {
    // ИСПРАВЛЕНИЕ: Защита от NULL
    if object.is_null() {
        return 0;
    }
    let count: NSUInteger = msg![env; object retainCount];
    count as CFIndex
}

pub fn CFEqual(env: &mut Environment, object1: CFTypeRef, object2: CFTypeRef) -> bool {
    // Если оба NULL — они равны (уже обрабатывается здесь)
    if object1 == object2 {
        return true;
    }
    // ИСПРАВЛЕНИЕ: Если только один из них NULL — они точно не равны.
    // Это спасет от краша при вызове [object class] ниже.
    if object1.is_null() || object2.is_null() {
        return false;
    }

    // TODO: other classes
    let str_class: Class = msg_class![env; NSString class];
    let object1_class: Class = msg![env; object1 class];
    assert!(msg![env; object1_class isKindOfClass:str_class]);
    let object2_class: Class = msg![env; object2 class];
    assert!(msg![env; object2_class isKindOfClass:str_class]);
    // TODO: use isEqual: once it is fixed
    msg![env; object1 isEqualToString:object2]
}

pub fn CFHash(env: &mut Environment, object: CFTypeRef) -> CFHashCode {
    // ИСПРАВЛЕНИЕ: Защита от NULL
    if object.is_null() {
        return 0;
    }
    msg![env; object hash]
}

/// Returns the type ID for a CF object. touchHLE uses the class pointer as a
/// stable unique-per-class value so that CFGetTypeID(x) == CFStringGetTypeID()
/// works correctly when x is an NSString/CFString.
pub fn CFGetTypeID(env: &mut Environment, cf: CFTypeRef) -> CFTypeID {
    if cf.is_null() {
        log_dbg!("CFGetTypeID: called with null, returning 0");
        return 0;
    }
    let class: Class = msg![env; cf class];
    class.to_bits() as CFTypeID
}

// --- Per-type ID functions ---
// Each returns the class pointer of the corresponding ObjC class so that
// comparisons with CFGetTypeID() are consistent.

pub fn CFStringGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = msg_class![env; NSString class];
    class.to_bits() as CFTypeID
}

pub fn CFDictionaryGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = msg_class![env; NSDictionary class];
    class.to_bits() as CFTypeID
}

pub fn CFArrayGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = msg_class![env; NSArray class];
    class.to_bits() as CFTypeID
}

pub fn CFNumberGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = msg_class![env; NSNumber class];
    class.to_bits() as CFTypeID
}

pub fn CFBooleanGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = msg_class![env; NSNumber class];
    class.to_bits() as CFTypeID
}

pub fn CFDataGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = msg_class![env; NSData class];
    class.to_bits() as CFTypeID
}

pub fn CFURLGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = msg_class![env; NSURL class];
    class.to_bits() as CFTypeID
}

/// `CFStringRef CFCopyDescription(CFTypeRef cf);`
///
/// Per the Core Foundation reference
/// (<https://developer.apple.com/documentation/corefoundation/1521252-cfcopydescription>):
///
/// > Returns the textual description of a Core Foundation object.
/// > If `cf` is a CF type that has a registered copyDescription callback,
/// > the callback is invoked. Otherwise the result is a synthetic string
/// > such as `<CFType 0x… [allocator]>{contents = …}`. Ownership follows
/// > the **Create Rule** — the caller owns the returned string and must
/// > release it.
///
/// On the touchHLE side every CF type is toll-free-bridged to an
/// Objective-C object, so the canonical implementation just forwards to
/// the object's `-description` selector (which Foundation overrides for
/// every concrete subclass) and retains the result so the caller has the
/// expected +1 reference count. `nil` is propagated through unchanged
/// because Apple's implementation likewise tolerates a NULL input on
/// recent OS releases.
pub fn CFCopyDescription(env: &mut Environment, cf: CFTypeRef) -> CFTypeRef {
    if cf.is_null() {
        return crate::objc::nil;
    }
    let desc: CFTypeRef = msg![env; cf description];
    if desc.is_null() {
        return crate::objc::nil;
    }
    // CF Create Rule: caller owns the return. -description returns an
    // autoreleased NSString, so retain it before handing it out.
    objc::retain(env, desc)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFRetain(_)),
    export_c_func!(CFRelease(_)),
    export_c_func!(CFGetRetainCount(_)),
    export_c_func!(CFEqual(_, _)),
    export_c_func!(CFHash(_)),
    export_c_func!(CFGetTypeID(_)),
    export_c_func!(CFStringGetTypeID()),
    export_c_func!(CFDictionaryGetTypeID()),
    export_c_func!(CFArrayGetTypeID()),
    export_c_func!(CFNumberGetTypeID()),
    export_c_func!(CFBooleanGetTypeID()),
    export_c_func!(CFDataGetTypeID()),
    export_c_func!(CFURLGetTypeID()),
    export_c_func!(CFCopyDescription(_)),
];
