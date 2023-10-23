/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSValue` class cluster, including `NSNumber`.

use super::NSUInteger;
use crate::objc::{
    autorelease, id, msg, msg_class, objc_classes, retain, Class, ClassExports, HostObject,
    NSZonePtr
};
use crate::mem::ConstPtr;

enum NSNumberHostObject {
    Bool(bool),
    UnsignedLongLong(u64),
    LongLong(i64),
    Double(f64),
}
impl HostObject for NSNumberHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSValue is an abstract class. None of the things it should provide are
// implemented here yet (TODO).
@implementation NSValue: NSObject

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

@end

// NSNumber is not an abstract class.
@implementation NSNumber: NSValue

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSNumberHostObject::Bool(false));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)numberWithBool:(bool)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithBool:value];
    autorelease(env, new)
}

+ (id)numberWithFloat:(f32)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithFloat:value];
    autorelease(env, new)
}

+ (id)numberWithDouble:(f64)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithDouble:value];
    autorelease(env, new)
}

+ (id)numberWithInt:(i32)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithInt:value];
    autorelease(env, new)
}

+ (id)numberWithLongLong:(i64)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithLongLong:value];
    autorelease(env, new)
}

+ (id)numberWithUnsignedLongLong:(u64)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithUnsignedLongLong:value];
    autorelease(env, new)
}

// TODO: types other than booleans and long longs

- (id)initWithBool:(bool)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::Bool(value);
    this
}

- (id)initWithFloat:(f32)value {
    msg![env; this initWithDouble: (value as f64)]
}

- (id)initWithDouble:(f64)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::Double(value);
    this
}

- (id)initWithInt:(i32)value {
    msg![env; this initWithLongLong: (value as i64)]
}

- (id)initWithLongLong:(i64)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::LongLong(value);
    this
}

- (id)initWithUnsignedLongLong:(u64)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::UnsignedLongLong(value);
    this
}

- (NSUInteger)hash {
    let value: i64 = msg![env; this longLongValue];
    super::hash_helper(&value)
}
- (bool)isEqualTo:(id)other {
    if this == other {
        return true;
    }
    let class: Class = msg_class![env; NSNumber class];
    if !msg![env; other isKindOfClass:class] {
        return false;
    }
    let &NSNumberHostObject::Bool(a) = env.objc.borrow(this) else {
        todo!();
    };
    let &NSNumberHostObject::Bool(b) = env.objc.borrow(other) else {
        todo!();
    };
    a == b
}

- (bool)boolValue {
    match env.objc.borrow::<NSNumberHostObject>(this) {
        NSNumberHostObject::Bool(b) => *b,
        NSNumberHostObject::UnsignedLongLong(u) => *u != 0,
        NSNumberHostObject::LongLong(l) => *l != 0,
        NSNumberHostObject::Double(d) => *d != 0.0,
    }
}

- (f64)doubleValue {
    match env.objc.borrow::<NSNumberHostObject>(this) {
        NSNumberHostObject::Bool(b) => *b as i32 as f64,
        NSNumberHostObject::UnsignedLongLong(u) => *u as f64,
        NSNumberHostObject::LongLong(l) => *l as f64,
        NSNumberHostObject::Double(d) => *d,
    }
}

- (f32)floatValue {
    let d: f64 = msg![env; this doubleValue];
    d as f32
}

- (i64)longLongValue {
    match env.objc.borrow::<NSNumberHostObject>(this) {
        NSNumberHostObject::Bool(b) => *b as i64,
        NSNumberHostObject::UnsignedLongLong(u) => *u as i64,
        NSNumberHostObject::LongLong(l) => *l,
        NSNumberHostObject::Double(d) => *d as i64,
    }
}

- (i32)intValue {
    let d: i64 = msg![env; this longLongValue];
    d as i32
}

-(ConstPtr<u8>)objCType {
    let ty = match env.objc.borrow::<NSNumberHostObject>(this) {
        NSNumberHostObject::Bool(_) => "B",
        NSNumberHostObject::UnsignedLongLong(_) => "Q",
        NSNumberHostObject::LongLong(_) => "q",
        NSNumberHostObject::Double(_) => "d",
    };
    let c_string = env.mem.alloc_and_write_cstr(ty.as_bytes());
    let length: NSUInteger = (ty.len() + 1).try_into().unwrap();
    // NSData will handle releasing the string (it is autoreleased)
    let _: id = msg_class![env; NSData dataWithBytesNoCopy:(c_string.cast_void())
                                                    length:length];
    c_string.cast_const()
}

// TODO: accessors etc

@end

};
