/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGPath`
use super::CGPoint;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::CFTypeRef;
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

// Note: on iOS SDK side this type is defined as a pointer to an opaque struct
pub(super) type CGMutablePathRef = CFTypeRef;

#[derive(Clone)]
pub struct CGMutablePathHostObject {
    pub path_elements: Vec<CGPathElement>,
}
impl HostObject for CGMutablePathHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGMutablePath: NSObject
@end

};

pub type CGPathElementType = i32;
pub const kCGPathElementMoveToPoint: CGPathElementType = 0;
pub const kCGPathElementAddLineToPoint: CGPathElementType = 1;

pub type CGLineCap = i32;
pub const kCGLineCapButt: CGLineCap = 0; // Default
pub const kCGLineCapRound: CGLineCap = 1;
pub const kCGLineCapSquare: CGLineCap = 2;

#[derive(Clone)]
pub struct CGPathElement {
    pub points: Vec<CGPoint>,
    pub r#type: CGPathElementType,
}

pub fn CGPathCreateMutable(env: &mut Environment) -> CGMutablePathRef {
    let path_elements: Vec<CGPathElement> = Vec::new();
    let host_obj = Box::new(CGMutablePathHostObject { path_elements });
    let class = env
        .objc
        .get_known_class("_touchHLE_CGMutablePath", &mut env.mem);
    env.objc.alloc_object(class, host_obj, &mut env.mem)
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CGPathCreateMutable())];
