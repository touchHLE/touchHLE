/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGContext.h`

use super::cg_affine_transform::CGAffineTransform;
use super::cg_image::CGImageRef;
use super::CGPoint;
use super::{cg_bitmap_context, cg_color, CGFloat, CGRect};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextGetHeight, CGBitmapContextGetWidth,
};
use crate::frameworks::core_graphics::cg_color::CGColorRef;
use crate::frameworks::core_graphics::cg_geometry::CGPointZero;
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

type CGInterpolationQuality = i32;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGContext seems to be a CFType-based type, but in our implementation those
// are just Objective-C types, so we need a class for it, but its name is not
// visible anywhere.
@implementation _touchHLE_CGContext: NSObject

- (())dealloc {
    let host_obj = env.objc.borrow::<CGContextHostObject>(this);
    let CGContextSubclass::CGBitmapContext(bitmap_data) = host_obj.subclass;
    if bitmap_data.data_is_owned {
        env.mem.free(bitmap_data.data);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

#[derive(Default)]
pub(super) struct CGContextPath {
    pub(super) subpaths: Vec<CGPathSubpath>,
}

pub(super) struct CGPathSubpath {
    pub(super) points: Vec<CGPoint>,
    pub(super) closed: bool,
}

pub(super) struct CGContextHostObject {
    pub(super) subclass: CGContextSubclass,
    pub(super) rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    pub(super) rgb_stroke_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    pub(super) line_width: CGFloat,
    pub(super) path: CGContextPath,
    /// Current transform.
    pub(super) transform: CGAffineTransform,
    // TODO: keep more states saved once they are implemented
    pub(super) state_stack: Vec<((CGFloat, CGFloat, CGFloat, CGFloat), CGAffineTransform)>,
}
impl HostObject for CGContextHostObject {}

pub(super) enum CGContextSubclass {
    CGBitmapContext(cg_bitmap_context::CGBitmapContextData),
}

pub type CGContextRef = CFTypeRef;

pub fn CGContextRelease(env: &mut Environment, c: CGContextRef) {
    if !c.is_null() {
        CFRelease(env, c);
    }
}
pub fn CGContextRetain(env: &mut Environment, c: CGContextRef) -> CGContextRef {
    if !c.is_null() {
        CFRetain(env, c)
    } else {
        c
    }
}

fn CGContextSetFillColorWithColor(env: &mut Environment, context: CGContextRef, color: CGColorRef) {
    let (r, g, b, a) = cg_color::to_rgba(&env.objc, color);
    CGContextSetRGBFillColor(env, context, r, g, b, a)
}

pub fn CGContextSetRGBFillColor(
    env: &mut Environment,
    context: CGContextRef,
    red: CGFloat,
    green: CGFloat,
    blue: CGFloat,
    alpha: CGFloat,
) {
    let color = (red, green, blue, alpha);
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .rgb_fill_color = color;
}

fn CGContextSetGrayFillColor(
    env: &mut Environment,
    context: CGContextRef,
    gray: CGFloat,
    alpha: CGFloat,
) {
    let color = (gray, gray, gray, alpha);
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .rgb_fill_color = color;
}

pub fn CGContextFillRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    cg_bitmap_context::fill_rect(env, context, rect, /* clear: */ false);
}

pub fn CGContextClearRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    cg_bitmap_context::fill_rect(env, context, rect, /* clear: */ true);
}

fn CGContextClipToRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    if rect.origin == CGPointZero
        && rect.size.height == CGBitmapContextGetHeight(env, context) as f32
        && rect.size.width == CGBitmapContextGetWidth(env, context) as f32
    {
        assert!(env
            .objc
            .borrow_mut::<CGContextHostObject>(context)
            .transform
            .is_identity());
        // All good, clipping is not needed!
        return;
    }
    todo!();
}

pub fn CGContextConcatCTM(
    env: &mut Environment,
    context: CGContextRef,
    transform: CGAffineTransform,
) {
    log_dbg!("CGContextConcatCTM({:?})", transform);
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = transform.concat(host_obj.transform);
}
pub fn CGContextGetCTM(env: &mut Environment, context: CGContextRef) -> CGAffineTransform {
    let res = env.objc.borrow::<CGContextHostObject>(context).transform;
    log_dbg!("CGContextGetCTM() => {:?}", res);
    res
}
pub fn CGContextRotateCTM(env: &mut Environment, context: CGContextRef, angle: CGFloat) {
    log_dbg!("CGContextRotateCTM({:?})", angle);
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = host_obj.transform.rotate(angle);
}
pub fn CGContextScaleCTM(env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat) {
    log_dbg!("CGContextScaleCTM({:?})", (x, y));
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = host_obj.transform.scale(x, y);
}
pub fn CGContextTranslateCTM(
    env: &mut Environment,
    context: CGContextRef,
    tx: CGFloat,
    ty: CGFloat,
) {
    log_dbg!("CGContextTranslateCTM({:?})", (tx, ty));
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = host_obj.transform.translate(tx, ty);
}

pub fn CGContextDrawImage(
    env: &mut Environment,
    context: CGContextRef,
    rect: CGRect,
    image: CGImageRef,
) {
    cg_bitmap_context::draw_image(env, context, rect, image);
}

fn CGContextSaveGState(env: &mut Environment, context: CGContextRef) {
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj
        .state_stack
        .push((host_obj.rgb_fill_color, host_obj.transform));
}

fn CGContextRestoreGState(env: &mut Environment, context: CGContextRef) {
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    let state = host_obj.state_stack.pop().unwrap();
    host_obj.rgb_fill_color = state.0;
    host_obj.transform = state.1;
}

fn CGContextSetInterpolationQuality(
    _env: &mut Environment,
    context: CGContextRef,
    quality: CGInterpolationQuality,
) {
    log!(
        "TODO: CGContextSetInterpolationQuality({:?}, {:?})",
        context,
        quality
    );
}

fn CGContextBeginPath(env: &mut Environment, context: CGContextRef) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .path
        .subpaths
        .clear();
}

fn CGContextMoveToPoint(env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .path
        .subpaths
        .push(CGPathSubpath {
            points: vec![CGPoint { x, y }],
            closed: false,
        });
}

fn CGContextAddLineToPoint(env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat) {
    let path = &mut env.objc.borrow_mut::<CGContextHostObject>(context).path;
    if let Some(last) = path.subpaths.last_mut() {
        if last.closed {
            let start = last.points[0];
            path.subpaths.push(CGPathSubpath {
                points: vec![start, CGPoint { x, y }],
                closed: false,
            });
        } else {
            last.points.push(CGPoint { x, y });
        }
    } else {
        path.subpaths.push(CGPathSubpath {
            points: vec![CGPoint { x, y }],
            closed: false,
        });
    }
}

fn CGContextClosePath(env: &mut Environment, context: CGContextRef) {
    if let Some(last) = env
        .objc
        .borrow_mut::<CGContextHostObject>(context)
        .path
        .subpaths
        .last_mut()
    {
        last.closed = true;
    }
}

fn CGContextAddRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    let x = rect.origin.x;
    let y = rect.origin.y;
    let w = rect.size.width;
    let h = rect.size.height;
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .path
        .subpaths
        .push(CGPathSubpath {
            points: vec![
                CGPoint { x, y },
                CGPoint { x: x + w, y },
                CGPoint { x: x + w, y: y + h },
                CGPoint { x, y: y + h },
            ],
            closed: true,
        });
}

fn CGContextAddEllipseInRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    let cx = rect.origin.x + rect.size.width / 2.0;
    let cy = rect.origin.y + rect.size.height / 2.0;
    let rx = rect.size.width / 2.0;
    let ry = rect.size.height / 2.0;
    let points = (0..64)
        .map(|i| {
            let theta = i as CGFloat * std::f32::consts::TAU / 64.0;
            CGPoint {
                x: cx + rx * theta.cos(),
                y: cy + ry * theta.sin(),
            }
        })
        .collect();
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .path
        .subpaths
        .push(CGPathSubpath {
            points,
            closed: true,
        });
}

fn CGContextDrawPath(env: &mut Environment, context: CGContextRef, mode: i32) {
    let paths = std::mem::take(
        &mut env
            .objc
            .borrow_mut::<CGContextHostObject>(context)
            .path
            .subpaths,
    );
    match mode {
        0 => cg_bitmap_context::draw_path(env, context, &paths, false, false),
        1 => cg_bitmap_context::draw_path(env, context, &paths, false, true),
        2 => cg_bitmap_context::draw_path(env, context, &paths, true, false),
        3 | 4 => {
            cg_bitmap_context::draw_path(env, context, &paths, false, mode == 4);
            cg_bitmap_context::draw_path(env, context, &paths, true, false);
        }
        _ => unimplemented!("CGContextDrawPath mode {}", mode),
    }
}

fn CGContextFillPath(env: &mut Environment, context: CGContextRef) {
    CGContextDrawPath(env, context, 0);
}

fn CGContextAddCurveToPoint(
    env: &mut Environment,
    context: CGContextRef,
    cp1x: CGFloat,
    cp1y: CGFloat,
    cp2x: CGFloat,
    cp2y: CGFloat,
    x: CGFloat,
    y: CGFloat,
) {
    let path = &mut env.objc.borrow_mut::<CGContextHostObject>(context).path;
    let Some(last) = path.subpaths.last_mut() else {
        return;
    };
    let Some(start) = last.points.last().copied() else {
        return;
    };
    for i in 1..=24 {
        let t = i as CGFloat / 24.0;
        let u = 1.0 - t;
        last.points.push(CGPoint {
            x: u * u * u * start.x
                + 3.0 * u * u * t * cp1x
                + 3.0 * u * t * t * cp2x
                + t * t * t * x,
            y: u * u * u * start.y
                + 3.0 * u * u * t * cp1y
                + 3.0 * u * t * t * cp2y
                + t * t * t * y,
        });
    }
}

fn CGContextAddQuadCurveToPoint(
    env: &mut Environment,
    context: CGContextRef,
    cpx: CGFloat,
    cpy: CGFloat,
    x: CGFloat,
    y: CGFloat,
) {
    let path = &mut env.objc.borrow_mut::<CGContextHostObject>(context).path;
    let Some(last) = path.subpaths.last_mut() else {
        return;
    };
    let Some(start) = last.points.last().copied() else {
        return;
    };
    for i in 1..=16 {
        let t = i as CGFloat / 16.0;
        let u = 1.0 - t;
        last.points.push(CGPoint {
            x: u * u * start.x + 2.0 * u * t * cpx + t * t * x,
            y: u * u * start.y + 2.0 * u * t * cpy + t * t * y,
        });
    }
}

fn CGContextEOFillPath(env: &mut Environment, context: CGContextRef) {
    CGContextDrawPath(env, context, 1);
}

fn CGContextStrokePath(env: &mut Environment, context: CGContextRef) {
    CGContextDrawPath(env, context, 2);
}

fn CGContextSetLineWidth(env: &mut Environment, context: CGContextRef, width: CGFloat) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .line_width = width;
}

fn CGContextSetGrayStrokeColor(
    env: &mut Environment,
    context: CGContextRef,
    gray: CGFloat,
    alpha: CGFloat,
) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .rgb_stroke_color = (gray, gray, gray, alpha);
}

fn CGContextSetRGBStrokeColor(
    env: &mut Environment,
    context: CGContextRef,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .rgb_stroke_color = (r, g, b, a);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGContextSetRGBStrokeColor(_, _, _, _, _)),
    export_c_func!(CGContextSetGrayStrokeColor(_, _, _)),
    export_c_func!(CGContextSetLineWidth(_, _)),
    export_c_func!(CGContextStrokePath(_)),
    export_c_func!(CGContextEOFillPath(_)),
    export_c_func!(CGContextAddQuadCurveToPoint(_, _, _, _, _)),
    export_c_func!(CGContextAddCurveToPoint(_, _, _, _, _, _, _)),
    export_c_func!(CGContextFillPath(_)),
    export_c_func!(CGContextDrawPath(_, _)),
    export_c_func!(CGContextAddEllipseInRect(_, _)),
    export_c_func!(CGContextAddRect(_, _)),
    export_c_func!(CGContextClosePath(_)),
    export_c_func!(CGContextAddLineToPoint(_, _, _)),
    export_c_func!(CGContextMoveToPoint(_, _, _)),
    export_c_func!(CGContextBeginPath(_)),
    export_c_func!(CGContextRetain(_)),
    export_c_func!(CGContextRelease(_)),
    export_c_func!(CGContextSetFillColorWithColor(_, _)),
    export_c_func!(CGContextSetRGBFillColor(_, _, _, _, _)),
    export_c_func!(CGContextSetGrayFillColor(_, _, _)),
    export_c_func!(CGContextFillRect(_, _)),
    export_c_func!(CGContextClearRect(_, _)),
    export_c_func!(CGContextClipToRect(_, _)),
    export_c_func!(CGContextConcatCTM(_, _)),
    export_c_func!(CGContextGetCTM(_)),
    export_c_func!(CGContextRotateCTM(_, _)),
    export_c_func!(CGContextScaleCTM(_, _, _)),
    export_c_func!(CGContextTranslateCTM(_, _, _)),
    export_c_func!(CGContextDrawImage(_, _, _)),
    export_c_func!(CGContextSaveGState(_)),
    export_c_func!(CGContextRestoreGState(_)),
    export_c_func!(CGContextSetInterpolationQuality(_, _)),
];
