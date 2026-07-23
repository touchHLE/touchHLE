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
pub mod ca_display_link;
pub mod ca_eagl_layer;
pub mod ca_layer;
pub mod ca_media_timing_function;
pub mod ca_transaction;

mod animation;
mod composition;

pub use composition::recomposite_if_necessary;

use crate::abi::{impl_GuestRet_for_large_struct, GuestArg};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::CGFloat;
use crate::mem::SafeRead;
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
        ca_display_link::CLASSES,
        ca_eagl_layer::CLASSES,
        ca_layer::CLASSES,
        ca_media_timing_function::CLASSES,
        ca_transaction::CLASSES,
    ],
    constant_exports: &[
        ca_animation::CONSTANTS,
        ca_layer::CONSTANTS,
        ca_media_timing_function::CONSTANTS,
        ca_transaction::CONSTANTS,
    ],
    function_exports: &[FUNCTIONS],
};

#[derive(Default)]
pub struct State {
    ca_media_timing_function: ca_media_timing_function::State,
    composition: composition::State,
}

#[derive(Default)]
pub struct ThreadLocalState {
    ca_transaction: ca_transaction::ThreadLocalState,
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

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct CATransform3D {
    pub m11: CGFloat,
    pub m12: CGFloat,
    pub m13: CGFloat,
    pub m14: CGFloat,
    pub m21: CGFloat,
    pub m22: CGFloat,
    pub m23: CGFloat,
    pub m24: CGFloat,
    pub m31: CGFloat,
    pub m32: CGFloat,
    pub m33: CGFloat,
    pub m34: CGFloat,
    pub m41: CGFloat,
    pub m42: CGFloat,
    pub m43: CGFloat,
    pub m44: CGFloat,
}
unsafe impl SafeRead for CATransform3D {}
impl GuestArg for CATransform3D {
    const REG_COUNT: usize = 16;

    fn from_regs(regs: &[u32]) -> Self {
        let mut values = [0.0; 16];
        for (idx, value) in values.iter_mut().enumerate() {
            *value = GuestArg::from_regs(&regs[idx..idx + 1]);
        }
        Self {
            m11: values[0],
            m12: values[1],
            m13: values[2],
            m14: values[3],
            m21: values[4],
            m22: values[5],
            m23: values[6],
            m24: values[7],
            m31: values[8],
            m32: values[9],
            m33: values[10],
            m34: values[11],
            m41: values[12],
            m42: values[13],
            m43: values[14],
            m44: values[15],
        }
    }

    fn to_regs(self, regs: &mut [u32]) {
        let values = [
            self.m11, self.m12, self.m13, self.m14, self.m21, self.m22, self.m23, self.m24,
            self.m31, self.m32, self.m33, self.m34, self.m41, self.m42, self.m43, self.m44,
        ];
        for (idx, value) in values.into_iter().enumerate() {
            value.to_regs(&mut regs[idx..idx + 1]);
        }
    }
}
impl_GuestRet_for_large_struct!(CATransform3D);

fn CATransform3DMakeScale(
    _env: &mut Environment,
    sx: CGFloat,
    sy: CGFloat,
    sz: CGFloat,
) -> CATransform3D {
    CATransform3D {
        m11: sx,
        m12: 0.0,
        m13: 0.0,
        m14: 0.0,
        m21: 0.0,
        m22: sy,
        m23: 0.0,
        m24: 0.0,
        m31: 0.0,
        m32: 0.0,
        m33: sz,
        m34: 0.0,
        m41: 0.0,
        m42: 0.0,
        m43: 0.0,
        m44: 1.0,
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CACurrentMediaTime()),
    export_c_func!(CATransform3DMakeScale(_, _, _)),
];
