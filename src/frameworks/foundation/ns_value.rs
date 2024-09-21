/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSValue` class cluster, including `NSNumber`.

use super::NSUInteger;
use crate::frameworks::foundation::ns_string::from_rust_string;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    autorelease, id, msg, msg_class, objc_classes, retain, Class, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

macro_rules! impl_AsValue {
    ($method_name:tt, $typ:tt) => {
        pub fn $method_name(&self) -> $typ {
            match self {
                // Cast to u8 is needed for float conversions
                NSNumberHostObject::Bool(x) => *x as u8 as _,
                NSNumberHostObject::UnsignedLongLong(x) => *x as _,
                NSNumberHostObject::Int(x) => *x as _,
                NSNumberHostObject::LongLong(x) => *x as _,
                NSNumberHostObject::Float(x) => *x as _,
                NSNumberHostObject::Double(x) => *x as _,
            }
        }
    };
}

#[derive(Debug)]
pub(super) enum NSNumberHostObject {
    Bool(bool),
    UnsignedLongLong(u64),
    Int(i32), // Also covers Integer since this is a 32 bit platform.
    LongLong(i64),
    Float(f32),
    Double(f64),
}
impl HostObject for NSNumberHostObject {}

impl NSNumberHostObject {
    fn as_bool(&self) -> bool {
        match self {
            NSNumberHostObject::Bool(x) => *x,
            NSNumberHostObject::UnsignedLongLong(x) => *x != 0,
            NSNumberHostObject::Int(x) => *x != 0,
            NSNumberHostObject::LongLong(x) => *x != 0,
            NSNumberHostObject::Float(x) => *x != 0.0,
            NSNumberHostObject::Double(x) => *x != 0.0,
        }
    }
    impl_AsValue!(as_int, i32);
    impl_AsValue!(as_long_long, i64);
    impl_AsValue!(as_unsigned_long_long, u64);
    impl_AsValue!(as_float, f32);
    impl_AsValue!(as_double, f64);
}

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

+ (id)numberWithInteger:(NSInteger)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithInteger:value];
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
    *env.objc.borrow_mut(this) = NSNumberHostObject::Float(value);
    this
}

- (id)initWithDouble:(f64)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::Double(value);
    this
}

- (id)initWithLongLong:(i64)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::LongLong(value);
    this
}

- (id)initWithInt:(i32)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::Int(value);
    this
}

- (id)initWithInteger:(NSInteger)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::Int(value);
    this
}

- (id)initWithUnsignedLongLong:(u64)value {
    *env.objc.borrow_mut(this) = NSNumberHostObject::UnsignedLongLong(value);
    this
}


- (bool)boolValue {
    env.objc.borrow::<NSNumberHostObject>(this).as_bool()
}

- (NSInteger)integerValue {
    env.objc.borrow::<NSNumberHostObject>(this).as_int()
}

- (i32)intValue {
    env.objc.borrow::<NSNumberHostObject>(this).as_int()
}

- (f32)floatValue {
    env.objc.borrow::<NSNumberHostObject>(this).as_float()
}

- (f64)doubleValue {
    env.objc.borrow::<NSNumberHostObject>(this).as_double()
}

- (i64)longLongValue {
    env.objc.borrow::<NSNumberHostObject>(this).as_long_long()
}

- (u64)unsignedLongLongValue {
    env.objc.borrow::<NSNumberHostObject>(this).as_unsigned_long_long()
}

- (id)description {
    let desc = match env.objc.borrow(this) {
        NSNumberHostObject::Bool(value) => from_rust_string(env, (*value as i32).to_string()),
        NSNumberHostObject::UnsignedLongLong(value) => from_rust_string(env, value.to_string()),
        NSNumberHostObject::Int(value) => from_rust_string(env, value.to_string()),
        NSNumberHostObject::LongLong(value) => from_rust_string(env, value.to_string()),
        NSNumberHostObject::Float(value) => from_rust_string(env, value.to_string()),
        NSNumberHostObject::Double(value) => from_rust_string(env, value.to_string())
    };
    autorelease(env, desc)
}

- (NSUInteger)hash {
    // The only requirement for [obj hash] is that values that compare equal
    // (via [obj isEqual] have the same hash. Hashing the underlying
    // bits works here.
    let value =
    match env.objc.borrow(this) {
        NSNumberHostObject::Bool(value) => *value as u64,
        NSNumberHostObject::UnsignedLongLong(value) => *value,
        NSNumberHostObject::Int(value) => *value as u64,
        NSNumberHostObject::LongLong(value) => *value as u64,
        NSNumberHostObject::Float(value) => value.to_bits() as u64,
        NSNumberHostObject::Double(value) => value.to_bits(),
    };
    super::hash_helper(&value)
}

- (bool)isEqualTo:(id)other {
    equality_helper(env, this, other)
}

// TODO: accessors etc

@end

};

fn equality_helper(env: &mut Environment, this: id, other: id) -> bool {
    if this == other {
        return true;
    }
    let class: Class = msg_class![env; NSNumber class];
    if !msg![env; other isKindOfClass:class] {
        return false;
    }
    match (env.objc.borrow(this), env.objc.borrow(other)) {
        (&NSNumberHostObject::Bool(a), &NSNumberHostObject::Bool(b)) => a == b,
        (&NSNumberHostObject::UnsignedLongLong(a), &NSNumberHostObject::UnsignedLongLong(b)) => {
            a == b
        }
        (&NSNumberHostObject::Int(a), &NSNumberHostObject::Int(b)) => a == b,
        (&NSNumberHostObject::LongLong(a), &NSNumberHostObject::LongLong(b)) => a == b,
        (&NSNumberHostObject::Float(a), &NSNumberHostObject::Float(b)) => a == b,
        (&NSNumberHostObject::Double(a), &NSNumberHostObject::Double(b)) => a == b,
        _ => todo!("Implement NSNumber comparisions of different types"),
    }
}
