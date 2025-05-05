/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIGraphics.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_graphics::cg_bitmap_context::CGBitmapContextCreate;
use crate::frameworks::core_graphics::cg_color_space::CGColorSpaceCreateDeviceRGB;
use crate::frameworks::core_graphics::cg_context::{
    CGContextRef, CGContextRelease, CGContextRetain,
};
use crate::frameworks::core_graphics::cg_image::{kCGImageAlphaPremultipliedLast, kCGImageByteOrder32Big};
use crate::frameworks::core_graphics::CGSize;
use crate::mem::{GuestUSize, MutVoidPtr};
use crate::objc::{id, nil};
use crate::Environment;

pub type UIImageRef = id;

#[derive(Default)]
pub(super) struct State {
    pub(super) context_stack: Vec<CGContextRef>,
}

pub fn UIGraphicsPushContext(env: &mut Environment, context: CGContextRef) {
    CGContextRetain(env, context);
    env.framework_state
        .uikit
        .ui_graphics
        .context_stack
        .push(context);
}
pub fn UIGraphicsPopContext(env: &mut Environment) {
    let context = env.framework_state.uikit.ui_graphics.context_stack.pop();
    CGContextRelease(env, context.unwrap());
}
pub fn UIGraphicsGetCurrentContext(env: &mut Environment) -> CGContextRef {
    env.framework_state
        .uikit
        .ui_graphics
        .context_stack
        .last()
        .copied()
        .unwrap_or(nil)
}
pub fn UIGraphicsBeginImageContext(env: &mut Environment, size: CGSize) {
    let width = size.width as GuestUSize;
    let height = size.height as GuestUSize;
    let color_space = CGColorSpaceCreateDeviceRGB(env);
    let context = CGBitmapContextCreate(
        env, MutVoidPtr::null(), width, height, 8, width * 4,
        color_space,
        kCGImageByteOrder32Big | kCGImageAlphaPremultipliedLast,
    );
    UIGraphicsPushContext(env, context);
}
pub fn UIGraphicsEndImageContext(env: &mut Environment) {
    UIGraphicsPopContext(env);
}

pub fn UIGraphicsGetImageFromCurrentImageContext(_env: &mut Environment) -> UIImageRef {
    nil
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(UIGraphicsPushContext(_)),
    export_c_func!(UIGraphicsPopContext()),
    export_c_func!(UIGraphicsGetCurrentContext()),
    export_c_func!(UIGraphicsBeginImageContext(_)),
    export_c_func!(UIGraphicsEndImageContext()),
    export_c_func!(UIGraphicsGetImageFromCurrentImageContext()),
];
