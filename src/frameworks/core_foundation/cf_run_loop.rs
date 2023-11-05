/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFRunLoop`.
//!
//! This is not even toll-free bridged to `NSRunLoop` in Apple's implementation,
//! but here it is the same type.

use sdl2_sys::u_long;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::objc::msg_class;
use crate::Environment;
use crate::frameworks::core_foundation::cf_allocator::CFAllocatorRef;
use crate::frameworks::core_foundation::CFIndex;
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::mem::{ConstVoidPtr, MutPtr};

pub type CFRunLoopRef = super::CFTypeRef;
pub type CFRunLoopMode = super::cf_string::CFStringRef;
pub type CFAbsoluteTime = CFTimeInterval;
pub type CFOptionFlags = u_long;

// typedef struct __CFRunLoopTimer CFRunLoopTimerRef;
pub type CFRunLoopTimerRef = super::CFTypeRef;

// typedef void (*CFRunLoopTimerCallBack)(CFRunLoopTimerRef timer, void *info);
pub type CFRunLoopTimerCallBack = super::CFTypeRef;

// typedef struct CFRunLoopTimerContext {
//     ...
// } CFRunLoopTimerContext;

// copyDescription, info, release, retain, version
pub type CFRunLoopTimerContext = super::CFTypeRef;

fn CFRunLoopGetCurrent(env: &mut Environment) -> CFRunLoopRef {
    msg_class![env; NSRunLoop currentRunLoop]
}

pub fn CFRunLoopGetMain(env: &mut Environment) -> CFRunLoopRef {
    msg_class![env; NSRunLoop mainRunLoop]
}

// TODO: Not sure what void (^block)(CFRunLoopTimerRef timer) is.
// CFRunLoopTimerRef CFRunLoopTimerCreateWithHandler(CFAllocatorRef allocator, CFAbsoluteTime fireDate, CFTimeInterval interval, CFOptionFlags flags, CFIndex order, void (^block)(CFRunLoopTimerRef timer));
pub fn CFRunLoopTimerCreateWithHandler(env: &mut Environment, allocator: CFAllocatorRef, fireDate: CFAbsoluteTime, interval: CFTimeInterval, flags: CFOptionFlags, order: CFIndex, timer: CFRunLoopTimerRef ) -> CFRunLoopRef {
    // TODO: Create a new CFRunLoopTimer
    msg_class![env; NSRunLoop currentRunLoop]
}

// CFRunLoopTimerRef CFRunLoopTimerCreate(CFAllocatorRef allocator, CFAbsoluteTime fireDate, CFTimeInterval interval, CFOptionFlags flags, CFIndex order, CFRunLoopTimerCallBack callout, CFRunLoopTimerContext *context);
pub fn CFRunLoopTimerCreate(env: &mut Environment, allocator: CFAllocatorRef, fireDate: CFAbsoluteTime, interval: CFTimeInterval, flags: CFOptionFlags, order: CFIndex,  callout: CFRunLoopTimerCallBack, context: MutPtr<CFRunLoopTimerContext>) -> CFRunLoopTimerRef {
    msg_class![env; NSRunLoop currentRunLoop]
}

// void CFRunLoopAddTimer(CFRunLoopRef rl, CFRunLoopTimerRef timer, CFRunLoopMode mode);
pub fn CFRunLoopAddTimer(env: &mut Environment, rl: CFRunLoopRef, timer: CFRunLoopTimerRef, mode: CFRunLoopMode) {

}

pub const kCFRunLoopCommonModes: &str = "kCFRunLoopCommonModes";
pub const kCFRunLoopDefaultMode: &str = "kCFRunLoopDefaultMode";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCFRunLoopCommonModes",
        HostConstant::NSString(kCFRunLoopCommonModes),
    ),
    (
        "_kCFRunLoopDefaultMode",
        HostConstant::NSString(kCFRunLoopDefaultMode),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFRunLoopGetCurrent()),
    export_c_func!(CFRunLoopGetMain()),
    export_c_func!(CFRunLoopTimerCreateWithHandler(_, _, _, _, _, _)),
    export_c_func!(CFRunLoopTimerCreate(_, _, _, _, _, _, _)),
    export_c_func!(CFRunLoopAddTimer(_, _, _))
];
