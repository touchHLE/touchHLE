/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFData` and `CFMutableData`.
//!
//! These are toll-free bridged to `NSData` and `NSMutableData` in Apple's
//! implementation. Here they are the same types.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::{CFIndex, CFRange};
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::foundation::{NSRange, NSUInteger};
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr};
use crate::objc::{id, msg, msg_class};
use crate::Environment;

pub type CFDataRef = super::CFTypeRef;

pub fn CFDataCreate(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    bytes: ConstPtr<u8>,
    length: CFIndex,
) -> CFDataRef {
    assert!(allocator == kCFAllocatorDefault); // unimplemented
    let bytes: ConstVoidPtr = bytes.cast();
    let length: NSUInteger = length.try_into().unwrap();
    let new: id = msg_class![env; NSData alloc];
    msg![env; new dataWithBytes:bytes length:length]
}

fn CFDataGetLength(env: &mut Environment, data: CFDataRef) -> CFIndex {
    let len: NSUInteger = msg![env; data length];
    len.try_into().unwrap()
}

fn CFDataGetBytePtr(env: &mut Environment, data: CFDataRef) -> ConstPtr<u8> {
    let ptr: ConstVoidPtr = msg![env; data bytes];
    ptr.cast()
}

fn CFDataGetBytes(env: &mut Environment, data: CFDataRef, range: CFRange, buffer: MutPtr<u8>) {
    let range = NSRange {
        location: range.location.try_into().unwrap(),
        length: range.length.try_into().unwrap(),
    };
    msg![env; data getBytes:buffer range:range]
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFDataCreate(_, _, _)),
    export_c_func!(CFDataGetLength(_)),
    export_c_func!(CFDataGetBytePtr(_)),
    export_c_func!(CFDataGetBytes(_, _, _)),
];
