//! `CFRunLoop`.
//!
//! This is not even toll-free bridged to `NSRunLoop` in Apple's implementation,
//! but here it is the same type.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::objc::msg_class;
use crate::Environment;

pub type CFRunLoopRef = super::CFTypeRef;

fn CFRunLoopGetCurrent(env: &mut Environment) -> CFRunLoopRef {
    msg_class![env; NSRunLoop currentRunLoop]
}

fn CFRunLoopGetMain(env: &mut Environment) -> CFRunLoopRef {
    msg_class![env; NSRunLoop mainRunLoop]
}

const kCFRunLoopCommonModes: &str = "kCFRunLoopCommonModes";

pub const CONSTANTS: ConstantExports = &[(
    "_kCFRunLoopCommonModes",
    HostConstant::NSString(kCFRunLoopCommonModes),
)];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFRunLoopGetCurrent()),
    export_c_func!(CFRunLoopGetMain()),
];
