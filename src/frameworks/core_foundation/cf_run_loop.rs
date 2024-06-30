/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFRunLoop`.
//!
//! This is not even toll-free bridged to `NSRunLoop` in Apple's implementation,
//! but here it is the same type.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::foundation::ns_run_loop::run_run_loop_single_iteration;
use crate::frameworks::foundation::ns_string;
use crate::objc::{msg, msg_class};
use crate::Environment;

pub type CFRunLoopRef = super::CFTypeRef;
pub type CFRunLoopMode = super::cf_string::CFStringRef;

fn CFRunLoopGetCurrent(env: &mut Environment) -> CFRunLoopRef {
    msg_class![env; NSRunLoop currentRunLoop]
}

pub fn CFRunLoopGetMain(env: &mut Environment) -> CFRunLoopRef {
    msg_class![env; NSRunLoop mainRunLoop]
}

fn CFRunLoopRunInMode(
    env: &mut Environment,
    mode: CFRunLoopMode,
    seconds: CFTimeInterval,
    _return_something: bool,
) -> i32 {
    let default_mode = ns_string::get_static_str(env, kCFRunLoopDefaultMode);
    let common_modes = ns_string::get_static_str(env, kCFRunLoopCommonModes);
    // TODO: handle other modes
    assert!(
        msg![env; mode isEqualToString:default_mode]
            || msg![env; mode isEqualToString:common_modes]
    );
    assert!(seconds <= 0.001);
    // TODO: we're currently supporting only the main run loop
    log_dbg!(
        "TODO: properly implement CFRunLoopRunInMode [current thread {}], running a single iteration of the main run loop",
        env.current_thread
    );
    let main_run_loop = CFRunLoopGetMain(env);
    run_run_loop_single_iteration(env, main_run_loop);
    1 // kCFRunLoopRunFinished
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
    export_c_func!(CFRunLoopRunInMode(_, _, _)),
];
