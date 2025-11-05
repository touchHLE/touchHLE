/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Animation framework.
//!
//! Useful resources:
//! - Apple's [Core Animation Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/Introduction/Introduction.html)

pub mod ca_animation;
pub mod ca_eagl_layer;
pub mod ca_layer;
pub mod ca_media_timing_function;

mod composition;
pub use composition::recomposite_if_necessary;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::libc::mach::thread_info::KERN_SUCCESS;
use crate::libc::mach::time::{mach_absolute_time, mach_timebase_info, struct_mach_timebase_info};
use crate::mem::guest_size_of;

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    // Core Animation is considered its own framework, but it technically lives
    // in a binary called QuartzCore, which does not contain anything else of
    // interest in iPhone OS 2 and 3. (iOS 5 adds Core Image to QuartzCore.)
    path: "/System/Library/Frameworks/QuartzCore.framework/QuartzCore",
    aliases: &[],
    class_exports: &[
        ca_animation::CLASSES,
        ca_eagl_layer::CLASSES,
        ca_layer::CLASSES,
        ca_media_timing_function::CLASSES,
    ],
    constant_exports: &[
        ca_animation::CONSTANTS,
        ca_layer::CONSTANTS,
        ca_media_timing_function::CONSTANTS,
    ],
    function_exports: &[FUNCTIONS],
};

#[derive(Default)]
pub struct State {
    composition: composition::State,
}

pub fn CACurrentMediaTime(env: &mut Environment) -> CFTimeInterval {
    let timebase_info_ptr = env
        .mem
        .alloc(guest_size_of::<struct_mach_timebase_info>())
        .cast();
    let return_value = mach_timebase_info(env, timebase_info_ptr);
    assert_eq!(return_value, KERN_SUCCESS);
    let timebase_info = env.mem.read(timebase_info_ptr);
    env.mem.free(timebase_info_ptr.cast_void());
    let time_abs = mach_absolute_time(env);
    let time_ns = time_abs * timebase_info.numerator as u64 / timebase_info.denominator as u64;
    time_ns as f64 / 1_000_000_000.0
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CACurrentMediaTime())];
