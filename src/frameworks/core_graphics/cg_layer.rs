/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGLayer.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::cg_bitmap_context::CGBitmapContextCreate;
use crate::frameworks::core_graphics::cg_context::CGContextRef;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::mem::MutVoidPtr;
use crate::objc::{nil, objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGLayerRef = CFTypeRef;

#[derive(Default)]
struct CGLayerHostObject {
    /// Off-screen bitmap context that backs this layer.
    context: CGContextRef,
    size: CGSize,
}
impl HostObject for CGLayerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGLayer: NSObject

- (())dealloc {
    let context = env.objc.borrow::<CGLayerHostObject>(this).context;
    if !context.is_null() {
        crate::frameworks::core_graphics::cg_context::CGContextRelease(env, context);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

// MARK: - Lifecycle

pub fn CGLayerRetain(env: &mut Environment, layer: CGLayerRef) -> CGLayerRef {
    if !layer.is_null() {
        CFRetain(env, layer)
    } else {
        layer
    }
}

pub fn CGLayerRelease(env: &mut Environment, layer: CGLayerRef) {
    if !layer.is_null() {
        CFRelease(env, layer);
    }
}

// MARK: - Creation

fn CGLayerCreateWithContext(
    env: &mut Environment,
    _context: CGContextRef,
    size: CGSize,
    _auxiliaryInfo: MutVoidPtr, // CFDictionaryRef — always NULL in practice
) -> CGLayerRef {
    let width = size.width as u32;
    let height = size.height as u32;

    if width == 0 || height == 0 {
        log!("Warning: CGLayerCreateWithContext called with zero size, returning nil");
        return nil;
    }

    // Determine bytes-per-row (4 bytes per pixel, RGBA).
    let bytes_per_row = width * 4;
    let bytes_total = bytes_per_row * height;
    let data: MutVoidPtr = env.mem.alloc(bytes_total).cast();

    // Create a backing bitmap context with the same colour space as the
    // reference context (we pass nil for colour space → uses device RGB).
    let layer_ctx = CGBitmapContextCreate(
        env,
        data,
        width as _,
        height as _,
        8, // bits per component
        bytes_per_row as _,
        nil,         // colorspace — device RGB
        0x0002_0008, // kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big
    );

    if layer_ctx.is_null() {
        log!("Warning: CGLayerCreateWithContext: failed to create backing context");
        return nil;
    }

    let class = env.objc.get_known_class("_touchHLE_CGLayer", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGLayerHostObject {
            context: layer_ctx,
            size,
        }),
        &mut env.mem,
    )
}

// MARK: - Accessors

fn CGLayerGetSize(env: &mut Environment, layer: CGLayerRef) -> CGSize {
    if layer.is_null() {
        return CGSize {
            width: 0.0,
            height: 0.0,
        };
    }
    env.objc.borrow::<CGLayerHostObject>(layer).size
}

fn CGLayerGetContext(env: &mut Environment, layer: CGLayerRef) -> CGContextRef {
    if layer.is_null() {
        return nil;
    }
    env.objc.borrow::<CGLayerHostObject>(layer).context
}

// MARK: - Drawing into a context

fn CGContextDrawLayerAtPoint(
    env: &mut Environment,
    context: CGContextRef,
    point: CGPoint,
    layer: CGLayerRef,
) {
    if layer.is_null() || context.is_null() {
        return;
    }
    let size = env.objc.borrow::<CGLayerHostObject>(layer).size;
    let rect = CGRect {
        origin: point,
        size,
    };
    CGContextDrawLayerInRect(env, context, rect, layer);
}

fn CGContextDrawLayerInRect(
    env: &mut Environment,
    context: CGContextRef,
    rect: CGRect,
    layer: CGLayerRef,
) {
    if layer.is_null() || context.is_null() {
        return;
    }

    // Obtain a CGImage snapshot of the layer's backing context and draw it.
    let layer_ctx = env.objc.borrow::<CGLayerHostObject>(layer).context;
    let cg_image = crate::frameworks::core_graphics::cg_bitmap_context::CGBitmapContextCreateImage(
        env, layer_ctx,
    );

    if cg_image.is_null() {
        log!("Warning: CGContextDrawLayerInRect: failed to create image from layer");
        return;
    }

    crate::frameworks::core_graphics::cg_context::CGContextDrawImage(env, context, rect, cg_image);

    crate::frameworks::core_graphics::cg_image::CGImageRelease(env, cg_image);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGLayerRetain(_)),
    export_c_func!(CGLayerRelease(_)),
    export_c_func!(CGLayerCreateWithContext(_, _, _)),
    export_c_func!(CGLayerGetSize(_)),
    export_c_func!(CGLayerGetContext(_)),
    export_c_func!(CGContextDrawLayerAtPoint(_, _, _)),
    export_c_func!(CGContextDrawLayerInRect(_, _, _)),
];
