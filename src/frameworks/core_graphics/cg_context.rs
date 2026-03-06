/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGContext.h`

use super::cg_affine_transform::CGAffineTransform;
use super::cg_image::CGImageRef;
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
pub type CGBlendMode = i32;

pub const kCGBlendModeNormal: CGBlendMode = 0;
pub const kCGBlendModeMultiply: CGBlendMode = 1;
pub const kCGBlendModeScreen: CGBlendMode = 2;
pub const kCGBlendModeOverlay: CGBlendMode = 3;
pub const kCGBlendModeDarken: CGBlendMode = 4;
pub const kCGBlendModeLighten: CGBlendMode = 5;
pub const kCGBlendModeColorDodge: CGBlendMode = 6;
pub const kCGBlendModeColorBurn: CGBlendMode = 7;
pub const kCGBlendModeSoftLight: CGBlendMode = 8;
pub const kCGBlendModeHardLight: CGBlendMode = 9;
pub const kCGBlendModeDifference: CGBlendMode = 10;
pub const kCGBlendModeExclusion: CGBlendMode = 11;
pub const kCGBlendModeHue: CGBlendMode = 12;
pub const kCGBlendModeSaturation: CGBlendMode = 13;
pub const kCGBlendModeColor: CGBlendMode = 14;
pub const kCGBlendModeLuminosity: CGBlendMode = 15;
pub const kCGBlendModeClear: CGBlendMode = 16;
pub const kCGBlendModeCopy: CGBlendMode = 17;
pub const kCGBlendModeSourceIn: CGBlendMode = 18;
pub const kCGBlendModeSourceOut: CGBlendMode = 19;
pub const kCGBlendModeSourceAtop: CGBlendMode = 20;
pub const kCGBlendModeDestinationOver: CGBlendMode = 21;
pub const kCGBlendModeDestinationIn: CGBlendMode = 22;
pub const kCGBlendModeDestinationOut: CGBlendMode = 23;
pub const kCGBlendModeDestinationAtop: CGBlendMode = 24;
pub const kCGBlendModeXOR: CGBlendMode = 25;
pub const kCGBlendModePlusDarker: CGBlendMode = 26;
pub const kCGBlendModePlusLighter: CGBlendMode = 27;

#[allow(non_camel_case_types)]
pub(super) struct _touchHLE_CGContextState {
    rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    transform: CGAffineTransform,
    alpha: CGFloat,
    blend_mode: CGBlendMode,
}

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

pub(super) struct CGContextHostObject {
    pub(super) subclass: CGContextSubclass,
    pub(super) rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    /// Current transform.
    pub(super) transform: CGAffineTransform,
    // TODO: keep more states saved once they are implemented
    pub(super) state_stack: Vec<_touchHLE_CGContextState>,
    pub(super) alpha: CGFloat,
    pub(super) blend_mode: CGBlendMode,
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

    host_obj.state_stack.push(_touchHLE_CGContextState {
        rgb_fill_color: host_obj.rgb_fill_color,
        transform: host_obj.transform,
        alpha: host_obj.alpha,
        blend_mode: host_obj.blend_mode,
    });
}

fn CGContextRestoreGState(env: &mut Environment, context: CGContextRef) {
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);

    let state = host_obj
        .state_stack
        .pop()
        .expect("CGContextRestoreGState called with empty state stack");

    host_obj.rgb_fill_color = state.rgb_fill_color;
    host_obj.transform = state.transform;

    if host_obj.alpha != state.alpha {
        let old_alpha = host_obj.alpha;
        host_obj.alpha = state.alpha;
        log!(
            "{} -> CGContextRestoreGState({:?}, alpha: {})",
            old_alpha,
            context,
            state.alpha
        );
    }
    if host_obj.blend_mode != state.blend_mode {
        let old_blend_mode = host_obj.blend_mode;
        host_obj.blend_mode = state.blend_mode;
        log!(
            "{} / {} -> CGContextRestoreGState({:?}, blend_mode: {} / {})",
            old_blend_mode,
            blend_mode_name(old_blend_mode),
            context,
            state.blend_mode,
            blend_mode_name(state.blend_mode)
        );
    }
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

pub fn CGContextSetAlpha(env: &mut Environment, context: CGContextRef, alpha: CGFloat) {
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    if host_obj.alpha != alpha {
        let old_alpha = host_obj.alpha;
        host_obj.alpha = alpha;
        log!(
            "{} -> CGContextSetAlpha({:?}, {})",
            old_alpha,
            context,
            alpha
        );
    }
}

pub fn CGContextSetBlendMode(
    env: &mut Environment,
    context: CGContextRef,
    blend_mode: CGBlendMode,
) {
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    if host_obj.blend_mode != blend_mode {
        let old_blend_mode = host_obj.blend_mode;
        host_obj.blend_mode = blend_mode;
        log!(
            "{} / {} -> CGContextSetBlendMode({:?}, {} / {})",
            old_blend_mode,
            blend_mode_name(old_blend_mode),
            context,
            blend_mode,
            blend_mode_name(blend_mode)
        );
    }
}

pub fn CGContextGetUserSpaceToDeviceSpaceTransform(
    env: &mut Environment,
    context: CGContextRef,
) -> CGAffineTransform {
    let ctm = CGContextGetCTM(env, context);
    let host_obj = env.objc.borrow::<CGContextHostObject>(context);
    #[allow(unreachable_patterns)]
    match &host_obj.subclass {
        CGContextSubclass::CGBitmapContext(data) => {
            let height = data.height as CGFloat;
            let flip = CGAffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: -1.0,
                tx: 0.0,
                ty: height,
            };
            flip.concat(ctm)
        }
        _ => ctm,
    }
}

// Helper function for logging CGBlendMode
pub fn blend_mode_name(mode: CGBlendMode) -> &'static str {
    match mode {
        kCGBlendModeNormal => "Normal",
        kCGBlendModeMultiply => "Multiply",
        kCGBlendModeScreen => "Screen",
        kCGBlendModeOverlay => "Overlay",
        kCGBlendModeDarken => "Darken",
        kCGBlendModeLighten => "Lighten",
        kCGBlendModeColorDodge => "ColorDodge",
        kCGBlendModeColorBurn => "ColorBurn",
        kCGBlendModeSoftLight => "SoftLight",
        kCGBlendModeHardLight => "HardLight",
        kCGBlendModeDifference => "Difference",
        kCGBlendModeExclusion => "Exclusion",
        kCGBlendModeHue => "Hue",
        kCGBlendModeSaturation => "Saturation",
        kCGBlendModeColor => "Color",
        kCGBlendModeLuminosity => "Luminosity",
        kCGBlendModeClear => "Clear",
        kCGBlendModeCopy => "Copy",
        kCGBlendModeSourceIn => "SourceIn",
        kCGBlendModeSourceOut => "SourceOut",
        kCGBlendModeSourceAtop => "SourceAtop",
        kCGBlendModeDestinationOver => "DestinationOver",
        kCGBlendModeDestinationIn => "DestinationIn",
        kCGBlendModeDestinationOut => "DestinationOut",
        kCGBlendModeDestinationAtop => "DestinationAtop",
        kCGBlendModeXOR => "XOR",
        kCGBlendModePlusDarker => "PlusDarker",
        kCGBlendModePlusLighter => "PlusLighter",
        _ => "Unknown",
    }
}

pub const FUNCTIONS: FunctionExports = &[
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
    export_c_func!(CGContextSetAlpha(_, _)),
    export_c_func!(CGContextSetBlendMode(_, _)),
    export_c_func!(CGContextGetUserSpaceToDeviceSpaceTransform(_)),
];
