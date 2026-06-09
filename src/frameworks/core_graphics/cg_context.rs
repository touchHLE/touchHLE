/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGContext.h`

use super::cg_affine_transform::{CGAffineTransform, CGAffineTransformIdentity};
use super::cg_bitmap_context::{
    CGBitmapContextDrawer, CGBitmapContextGetHeight, CGBitmapContextGetWidth,
};
use super::cg_color::CGColorRef;
use super::cg_color_space::{
    kCGColorSpaceModelMonochrome, kCGColorSpaceModelRGB, CGColorSpaceGetModel, CGColorSpaceRef,
};
use super::cg_font::{CGFontHostObject, CGFontRef, CGFontRelease, CGFontRetain, CGGlyph};
use super::cg_geometry::CGPointZero;
use super::cg_image::CGImageRef;
use super::{cg_bitmap_context, cg_color, CGFloat, CGRect, CGSize};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::uikit;
use crate::mem::{ConstPtr, GuestUSize};
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

type CGInterpolationQuality = i32;

type CGTextDrawingMode = i32;
const kCGTextFill: CGTextDrawingMode = 0;
const kCGTextFillStroke: CGTextDrawingMode = 2;

pub type CGBlendMode = i32;
pub const kCGBlendModeNormal: CGBlendMode = 0;
pub const kCGBlendModeMultiply: CGBlendMode = 1;
pub const kCGBlendModeScreen: CGBlendMode = 2;
#[allow(unused)]
pub const kCGBlendModeOverlay: CGBlendMode = 3;
pub const kCGBlendModeDarken: CGBlendMode = 4;
pub const kCGBlendModeLighten: CGBlendMode = 5;

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
    CGFontRelease(env, host_obj.font);

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

// TODO: keep more states saved once they are implemented
type ContextState = (
    (CGFloat, CGFloat, CGFloat, CGFloat), // RGB fill color
    CGAffineTransform,                    // transform
    CGFontRef,                            // font
    CGFloat,                              // font size
    CGBlendMode,                          // blend mode
);

pub(super) struct CGContextHostObject {
    pub(super) subclass: CGContextSubclass,
    pub(super) rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    pub(super) font: CGFontRef,
    pub(super) font_size: CGFloat,
    /// Current transform.
    pub(super) transform: CGAffineTransform,
    pub(super) blend_mode: CGBlendMode,
    /// Text transform.
    pub(super) text_transform: Option<CGAffineTransform>,
    pub(super) state_stack: Vec<ContextState>,
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

fn CGContextSetBlendMode(env: &mut Environment, context: CGContextRef, blend_mode: CGBlendMode) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .blend_mode = blend_mode;
}

fn CGContextSetFillColorSpace(
    env: &mut Environment,
    _context: CGContextRef,
    space: CGColorSpaceRef,
) {
    let color_model = CGColorSpaceGetModel(env, space);
    assert!(color_model == kCGColorSpaceModelMonochrome || color_model == kCGColorSpaceModelRGB);
    // TODO
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

fn CGContextSetGrayStrokeColor(
    _env: &mut Environment,
    context: CGContextRef,
    gray: CGFloat,
    alpha: CGFloat,
) {
    log!(
        "TODO: CGContextSetGrayStrokeColor({:?}, {}, {})",
        context,
        gray,
        alpha,
    );
}
fn CGContextSetRGBStrokeColor(
    _env: &mut Environment,
    context: CGContextRef,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
) {
    log!(
        "TODO: CGContextSetRGBStrokeColor({:?}, {}, {}, {}, {})",
        context,
        r,
        g,
        b,
        a
    );
}

fn CGContextSetShadowWithColor(
    _env: &mut Environment,
    context: CGContextRef,
    offset: CGSize,
    blur: CGFloat,
    color: CGColorRef,
) {
    log!(
        "TODO: CGContextSetShadowWithColor({:?}, {}, {}, {:?})",
        context,
        offset,
        blur,
        color
    );
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
    host_obj.state_stack.push((
        host_obj.rgb_fill_color,
        host_obj.transform,
        host_obj.font,
        host_obj.font_size,
        host_obj.blend_mode,
    ));
    CGFontRetain(env, env.objc.borrow::<CGContextHostObject>(context).font);
}

fn CGContextRestoreGState(env: &mut Environment, context: CGContextRef) {
    // We need to release _old_ font, there are 2 cases:
    // - font hasn't been set between save/restore -> this release corresponds
    // the font retain from save
    // - font has been set between save/restore -> we need to release old font
    // retained on the set
    CGFontRelease(env, env.objc.borrow::<CGContextHostObject>(context).font);
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    let state = host_obj.state_stack.pop().unwrap();
    host_obj.rgb_fill_color = state.0;
    host_obj.transform = state.1;
    host_obj.font = state.2;
    host_obj.font_size = state.3;
    host_obj.blend_mode = state.4;
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
fn CGContextSetAllowsAntialiasing(_env: &mut Environment, context: CGContextRef, allow: bool) {
    log!(
        "TODO: CGContextSetAllowsAntialiasing({:?}, {})",
        context,
        allow
    );
}

fn CGContextSetFont(env: &mut Environment, context: CGContextRef, font: CGFontRef) {
    CGFontRetain(env, font);
    let old_font = env.objc.borrow_mut::<CGContextHostObject>(context).font;
    CGFontRelease(env, old_font);
    env.objc.borrow_mut::<CGContextHostObject>(context).font = font;
}

fn CGContextSetFontSize(env: &mut Environment, context: CGContextRef, size: CGFloat) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .font_size = size;
}

fn CGContextSetTextDrawingMode(
    _env: &mut Environment,
    _context: CGContextRef,
    mode: CGTextDrawingMode,
) {
    assert!(mode == kCGTextFill || mode == kCGTextFillStroke); // TODO: support other modes
}

fn CGContextSetTextMatrix(
    env: &mut Environment,
    context: CGContextRef,
    transform: CGAffineTransform,
) {
    log_dbg!("CGContextSetTextMatrix({:?})", transform);
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .text_transform = Some(transform);
}

fn CGContextShowGlyphsAtPoint(
    env: &mut Environment,
    context: CGContextRef,
    x: CGFloat,
    y: CGFloat,
    glyphs: ConstPtr<CGGlyph>,
    count: GuestUSize,
) {
    let mut glyph_ids = Vec::new();
    for i in 0..count {
        let glyph_id = env.mem.read(glyphs + i);
        glyph_ids.push(rusttype::GlyphId(glyph_id));
    }

    let font = env.objc.borrow::<CGContextHostObject>(context).font;
    let font_size = env.objc.borrow::<CGContextHostObject>(context).font_size;
    let text_transform = env
        .objc
        .borrow::<CGContextHostObject>(context)
        .text_transform
        .unwrap_or(CGAffineTransformIdentity);

    let font = &env.objc.borrow::<CGFontHostObject>(font).font;

    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let fill_color = drawer.rgb_fill_color();

    font.draw_glyphs(
        font_size,
        glyph_ids,
        (x, y),
        text_transform,
        |raster_glyph| {
            uikit::ui_font::draw_font_glyph(
                &mut drawer,
                raster_glyph,
                fill_color,
                /* clip_x: */ None,
                /* clip_y: */ None,
            )
        },
    );
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGContextRetain(_)),
    export_c_func!(CGContextRelease(_)),
    export_c_func!(CGContextSetBlendMode(_, _)),
    export_c_func!(CGContextSetFillColorSpace(_, _)),
    export_c_func!(CGContextSetFillColorWithColor(_, _)),
    export_c_func!(CGContextSetRGBFillColor(_, _, _, _, _)),
    export_c_func!(CGContextSetGrayFillColor(_, _, _)),
    export_c_func!(CGContextSetGrayStrokeColor(_, _, _)),
    export_c_func!(CGContextSetRGBStrokeColor(_, _, _, _, _)),
    export_c_func!(CGContextSetShadowWithColor(_, _, _, _)),
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
    export_c_func!(CGContextSetAllowsAntialiasing(_, _)),
    export_c_func!(CGContextSetFont(_, _)),
    export_c_func!(CGContextSetFontSize(_, _)),
    export_c_func!(CGContextSetTextDrawingMode(_, _)),
    export_c_func!(CGContextSetTextMatrix(_, _)),
    export_c_func!(CGContextShowGlyphsAtPoint(_, _, _, _, _)),
];
