/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGContext.h`

use owned_ttf_parser::GlyphId;
use super::cg_affine_transform::CGAffineTransform;
use super::cg_image::CGImageRef;
use super::{cg_bitmap_context, CGFloat, CGPoint, CGRect, CGSize};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextGetHeight, CGBitmapContextGetWidth,
};
use crate::frameworks::core_graphics::cg_geometry::CGPointZero;
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;
use crate::frameworks::core_graphics::cg_color::CGColorRef;
use crate::frameworks::core_graphics::cg_font::{glyphs_at_point, CGFontRef, CGGlyph};
use crate::mem::{ConstPtr, GuestUSize};

type CGInterpolationQuality = i32;
pub type CGTextDrawingMode = i32;
// TODO: find constant values for this enum

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

#[derive(Copy, Clone)]
pub(super) struct CGContextState {
    pub(super) rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    pub(super) rgb_stroke_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    pub(super) line_width: CGFloat,
    /// Current transform.
    pub(super) transform: CGAffineTransform,
    pub(super) font_size: CGFloat,
    pub(super) text_font: CGFontRef,
    pub(super) text_drawing_mode: CGTextDrawingMode,
    pub(super) shadow_offset: CGSize,
    pub(super) shadow_blur: CGFloat,
    pub(super) shadow_color: CGColorRef,
}

pub(super) struct CGContextHostObject {
    pub(super) subclass: CGContextSubclass,
    pub(super) state: CGContextState,
    // TODO: keep more states saved once they are implemented
    pub(super) state_stack: Vec<CGContextState>,
    // TODO: according to documentation these aren't part of state and aren't affected by pop/push, check if true?
    pub(super) text_matrix: CGAffineTransform,
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

pub fn CGContextSetLineWidth(
    env: &mut Environment,
    context: CGContextRef,
    line_width: CGFloat,
) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .state
        .line_width = line_width;
}

pub fn CGContextSetRGBStrokeColor(
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
        .state
        .rgb_stroke_color = color;
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
        .state
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
        .state
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
            .state
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
    host_obj.state.transform = transform.concat(host_obj.state.transform);
}
pub fn CGContextGetCTM(env: &mut Environment, context: CGContextRef) -> CGAffineTransform {
    let res = env.objc.borrow::<CGContextHostObject>(context).state.transform;
    log_dbg!("CGContextGetCTM() => {:?}", res);
    res
}
pub fn CGContextRotateCTM(env: &mut Environment, context: CGContextRef, angle: CGFloat) {
    log_dbg!("CGContextRotateCTM({:?})", angle);
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.state.transform = host_obj.state.transform.rotate(angle);
}
pub fn CGContextScaleCTM(env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat) {
    log_dbg!("CGContextScaleCTM({:?})", (x, y));
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.state.transform = host_obj.state.transform.scale(x, y);
}
pub fn CGContextTranslateCTM(
    env: &mut Environment,
    context: CGContextRef,
    tx: CGFloat,
    ty: CGFloat,
) {
    log_dbg!("CGContextTranslateCTM({:?})", (tx, ty));
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.state.transform = host_obj.state.transform.translate(tx, ty);
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
        .push(host_obj.state);
}

fn CGContextRestoreGState(env: &mut Environment, context: CGContextRef) {
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    let state = host_obj.state_stack.pop().unwrap();
    host_obj.state = state;
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

pub fn CGContextGetClipBoundingBox(
    _env: &mut Environment,
    _context: CGContextRef,
) -> CGRect {
    CGRect::default()
}

pub fn CGContextSetTextMatrix(env: &mut Environment, context: CGContextRef, matrix: CGAffineTransform) {
    env.objc.borrow_mut::<CGContextHostObject>(context).text_matrix = matrix;
}

pub fn CGContextSetFont(env: &mut Environment, context: CGContextRef, font: CGFontRef) {
    env.objc.borrow_mut::<CGContextHostObject>(context).state.text_font = font;
}

pub fn CGContextSetFontSize(env: &mut Environment, context: CGContextRef, font_size: CGFloat) {
    env.objc.borrow_mut::<CGContextHostObject>(context).state.font_size = font_size;
}

pub fn CGContextSetTextDrawingMode(env: &mut Environment, context: CGContextRef, mode: CGTextDrawingMode) {
    env.objc.borrow_mut::<CGContextHostObject>(context).state.text_drawing_mode = mode;
}

pub fn CGContextSetShadowWithColor(env: &mut Environment, context: CGContextRef, offset: CGSize, blur: CGFloat, color: CGColorRef) {
    env.objc.borrow_mut::<CGContextHostObject>(context).state.shadow_offset = offset;
    env.objc.borrow_mut::<CGContextHostObject>(context).state.shadow_blur = blur;
    env.objc.borrow_mut::<CGContextHostObject>(context).state.shadow_color = color;
}

pub fn CGContextShowGlyphsAtPoint(env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat, glyphs: ConstPtr<CGGlyph>, count: GuestUSize) {
    let context = env.objc.borrow::<CGContextHostObject>(context);
    let glyphs = (0..count)
        .map(|i| env.mem.read(glyphs+i))
        .map(|g| GlyphId(g))
        .collect::<Vec<_>>();
    glyphs_at_point(env, context.state.text_font, &glyphs, CGPoint {
        x, y
    })
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGContextRetain(_)),
    export_c_func!(CGContextRelease(_)),
    export_c_func!(CGContextSetLineWidth(_, _)),
    export_c_func!(CGContextSetRGBFillColor(_, _, _, _, _)),
    export_c_func!(CGContextSetRGBStrokeColor(_, _, _, _, _)),
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
    export_c_func!(CGContextGetClipBoundingBox(_)),
    export_c_func!(CGContextSetTextMatrix(_, _)),
    export_c_func!(CGContextSetFont(_, _)),
    export_c_func!(CGContextSetFontSize(_, _)),
    export_c_func!(CGContextSetTextDrawingMode(_, _)),
    export_c_func!(CGContextSetShadowWithColor(_, _, _, _)),
    export_c_func!(CGContextShowGlyphsAtPoint(_, _, _, _, _)),
];
