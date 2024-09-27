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

mod animation;
mod composition;

pub use composition::recomposite_if_necessary;

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::Environment;
use std::time::Instant;

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
    ca_media_timing_function: ca_media_timing_function::State,
    composition: composition::State,
}

// This function should call mach_absolute_time() and convert the result into
// seconds. Since in our implementation, mach_absolute_time() returns, in
// nanoseconds, Instant::now, we can just do the same in seconds and save
// the calls to the guest functions.
pub fn CACurrentMediaTime(env: &mut Environment) -> CFTimeInterval {
    Instant::now()
        .duration_since(env.startup_time)
        .as_secs_f64()
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CACurrentMediaTime())];
