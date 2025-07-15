/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFNumber`.
//!
//! This is toll-free bridged to `NSNumber` in Apple's implementation.
//! Here it is the same type.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::{CFComparisonResult, CFIndex, CFTypeRef};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_value::is_conversion_lossless;
use crate::mem::{ConstVoidPtr, MutVoidPtr};
use crate::objc::{id, msg, msg_class};
use crate::Environment;

pub type CFNumberType = CFIndex;
pub const kCFNumberSInt8Type: CFNumberType = 1;
pub const kCFNumberSInt16Type: CFNumberType = 2;
pub const kCFNumberSInt32Type: CFNumberType = 3;
pub const kCFNumberSInt64Type: CFNumberType = 4;
pub const kCFNumberFloat32Type: CFNumberType = 5;
pub const kCFNumberFloat64Type: CFNumberType = 6;
pub const kCFNumberCharType: CFNumberType = 7;
pub const kCFNumberShortType: CFNumberType = 8;
pub const kCFNumberIntType: CFNumberType = 9;
pub const kCFNumberLongLongType: CFNumberType = 11;
pub const kCFNumberFloatType: CFNumberType = 12;
pub const kCFNumberDoubleType: CFNumberType = 13;

type CFNumberRef = CFTypeRef;
// Note: on iOS SDK side this type is defined as a pointer to an opaque struct
type CFBooleanRef = CFNumberRef;

fn CFNumberCreate(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    type_: CFNumberType,
    value_ptr: ConstVoidPtr,
) -> CFNumberRef {
    // TODO: unique some common numbers to improve performance
    assert_eq!(allocator, kCFAllocatorDefault); // unimplemented
    log_dbg!("CFNumberCreate type {}", type_);
    let num = msg_class![env; NSNumber alloc];
    match type_ {
        kCFNumberSInt32Type | kCFNumberIntType => {
            let val: i32 = env.mem.read(value_ptr.cast());
            msg![env; num initWithInt:val]
        }
        kCFNumberSInt8Type | kCFNumberCharType => {
            let val: i8 = env.mem.read(value_ptr.cast());
            msg![env; num initWithChar:val]
        }
        kCFNumberSInt16Type | kCFNumberShortType => {
            let val: i16 = env.mem.read(value_ptr.cast());
            msg![env; num initWithShort:val]
        }
        kCFNumberFloat32Type | kCFNumberFloatType => {
            let val: f32 = env.mem.read(value_ptr.cast());
            msg![env; num initWithFloat:val]
        }
        kCFNumberFloat64Type | kCFNumberDoubleType => {
            let val: f64 = env.mem.read(value_ptr.cast());
            msg![env; num initWithDouble:val]
        }
        kCFNumberSInt64Type | kCFNumberLongLongType => {
            let val: i64 = env.mem.read(value_ptr.cast());
            msg![env; num initWithLongLong:val]
        }
        _ => unimplemented!("type {}", type_),
    }
}

fn CFNumberGetValue(
    env: &mut Environment,
    num: CFNumberRef,
    type_: CFNumberType,
    value_ptr: MutVoidPtr,
) -> bool {
    match type_ {
        kCFNumberSInt32Type | kCFNumberIntType => {
            let val: i32 = msg![env; num intValue];
            env.mem.write(value_ptr.cast(), val);
            is_conversion_lossless(env, num, type_)
        }
        kCFNumberSInt8Type | kCFNumberCharType => {
            let val: i8 = msg![env; num charValue];
            env.mem.write(value_ptr.cast(), val);
            is_conversion_lossless(env, num, type_)
        }
        kCFNumberSInt16Type | kCFNumberShortType => {
            let val: i16 = msg![env; num shortValue];
            env.mem.write(value_ptr.cast(), val);
            is_conversion_lossless(env, num, type_)
        }
        kCFNumberFloat32Type | kCFNumberFloatType => {
            let val: f32 = msg![env; num floatValue];
            env.mem.write(value_ptr.cast(), val);
            is_conversion_lossless(env, num, type_)
        }
        _ => unimplemented!("type {}", type_),
    }
}

fn CFNumberCompare(
    env: &mut Environment,
    num1: CFNumberRef,
    num2: CFNumberRef,
    context: MutVoidPtr,
) -> CFComparisonResult {
    assert!(context.is_null()); // always NULL according to the docs
    msg![env; num1 compare:num2]
}

fn CFBooleanGetValue(env: &mut Environment, boolean: CFBooleanRef) -> bool {
    msg![env; boolean boolValue]
}

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCFBooleanFalse",
        HostConstant::Custom(|env| {
            let num = msg_class![env; NSNumber alloc];
            let num: id = msg![env; num initWithBool:false];
            // Apparently, it's a pointer to pointer
            env.mem.alloc_and_write(num).cast_void().cast_const()
        }),
    ),
    (
        "_kCFBooleanTrue",
        HostConstant::Custom(|env| {
            let num = msg_class![env; NSNumber alloc];
            let num: id = msg![env; num initWithBool:true];
            // Apparently, it's a pointer to pointer
            env.mem.alloc_and_write(num).cast_void().cast_const()
        }),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFNumberCreate(_, _, _)),
    export_c_func!(CFNumberGetValue(_, _, _)),
    export_c_func!(CFNumberCompare(_, _, _)),
    export_c_func!(CFBooleanGetValue(_)),
];
