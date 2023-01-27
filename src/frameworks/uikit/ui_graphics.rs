//! `UIGraphics.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_graphics::cg_context::{
    CGContextRef, CGContextRelease, CGContextRetain,
};
use crate::objc::nil;
use crate::Environment;

#[derive(Default)]
pub(super) struct State {
    pub(super) context_stack: Vec<CGContextRef>,
}

fn UIGraphicsPushContext(env: &mut Environment, context: CGContextRef) {
    CGContextRetain(env, context);
    env.framework_state
        .uikit
        .ui_graphics
        .context_stack
        .push(context);
}
fn UIGraphicsPopContext(env: &mut Environment) {
    let context = env.framework_state.uikit.ui_graphics.context_stack.pop();
    CGContextRelease(env, context.unwrap());
}
fn UIGraphicsGetCurrentContext(env: &mut Environment) -> CGContextRef {
    env.framework_state
        .uikit
        .ui_graphics
        .context_stack
        .last()
        .copied()
        .unwrap_or(nil)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(UIGraphicsPushContext(_)),
    export_c_func!(UIGraphicsPopContext()),
    export_c_func!(UIGraphicsGetCurrentContext()),
];
