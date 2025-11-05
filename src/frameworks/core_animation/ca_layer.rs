/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CALayer`.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::cg_affine_transform::{
    CGAffineTransform, CGAffineTransformIdentity,
};
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextCreate, CGBitmapContextGetHeight, CGBitmapContextGetWidth,
};
use crate::frameworks::core_graphics::cg_color::{CGColorHostObject, CGColorRef};
use crate::frameworks::core_graphics::cg_color_space::CGColorSpaceCreateDeviceRGB;
use crate::frameworks::core_graphics::cg_context::{
    CGContextClearRect, CGContextRef, CGContextRelease, CGContextTranslateCTM,
};
use crate::frameworks::core_graphics::cg_image::{
    kCGImageAlphaPremultipliedLast, kCGImageByteOrder32Big,
};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string;
use crate::mem::{GuestUSize, Ptr};
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, ObjC,
};
use crate::Environment;
use std::collections::HashMap;

pub(super) struct CALayerHostObject {
    /// Possibly nil, usually a UIView. This is a weak reference.
    delegate: id,
    /// Sublayers in back-to-front order. These are strong references.
    pub(super) sublayers: Vec<id>,
    /// The superlayer. This is a weak reference.
    superlayer: id,
    pub(super) bounds: CGRect,
    pub(super) position: CGPoint,
    pub(super) anchor_point: CGPoint,
    pub(super) affine_transform: CGAffineTransform,
    pub(super) hidden: bool,
    pub(super) opaque: bool,
    pub(super) opacity: f32,
    pub(super) background_color: Option<CGColorHostObject>,
    pub(super) corner_radius: CGFloat,
    pub(super) needs_display: bool,
    /// `CGImageRef*`
    pub(super) contents: id,
    /// For CAEAGLLayer only
    pub(super) drawable_properties: id,
    /// For CAEAGLLayer only (internal state for compositor)
    pub(super) presented_pixels: Option<(Vec<u8>, u32, u32)>,
    /// Internal, only exposed when calling `drawLayer:inContext:`
    pub(super) cg_context: Option<CGContextRef>,
    /// Internal state for compositor
    pub(super) gles_texture: Option<crate::gles::gles11_raw::types::GLuint>,
    /// Internal state for compositor
    pub(super) gles_texture_is_up_to_date: bool,
}
impl HostObject for CALayerHostObject {}

impl CALayerHostObject {
    /// Internal helper method: generate a transformation matrix to transform
    /// from the superlayer's co-ordinate space (the space that the layer's
    /// position is specified in) to the layer's internal co-ordinate space
    /// (the space that the layer's bounds and its sublayers' positions are
    /// specified in).
    pub(super) fn superlayer_to_layer_transform(&self) -> CGAffineTransform {
        CGAffineTransform::make_translation(-self.bounds.origin.x, -self.bounds.origin.y)
            .concat(CGAffineTransform::make_translation(
                -self.bounds.size.width * self.anchor_point.x,
                -self.bounds.size.height * self.anchor_point.y,
            ))
            .concat(self.affine_transform)
            .concat(CGAffineTransform::make_translation(
                self.position.x,
                self.position.y,
            ))
    }
}

pub const kCAFilterLinear: &str = "kCAFilterLinear";
pub const kCAFilterNearest: &str = "kCAFilterNearest";
pub const kCAFilterTrilinear: &str = "kCAFilterTrilinear";

pub const CONSTANTS: ConstantExports = &[
    ("_kCAFilterLinear", HostConstant::NSString(kCAFilterLinear)),
    (
        "_kCAFilterNearest",
        HostConstant::NSString(kCAFilterNearest),
    ),
    (
        "_kCAFilterTrilinear",
        HostConstant::NSString(kCAFilterTrilinear),
    ),
];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CALayer: NSObject

+ (id)alloc {
    let host_object = Box::new(CALayerHostObject {
        delegate: nil,
        sublayers: Vec::new(),
        superlayer: nil,
        bounds: CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width: 0.0, height: 0.0 }
        },
        position: CGPoint { x: 0.0, y: 0.0 },
        anchor_point: CGPoint { x: 0.5, y: 0.5 },
        affine_transform: CGAffineTransformIdentity,
        hidden: false,
        opaque: false,
        opacity: 1.0,
        background_color: None, // transparency
        corner_radius: 0.0,
        needs_display: false,
        contents: nil,
        drawable_properties: nil,
        presented_pixels: None,
        cg_context: None,
        gles_texture: None,
        gles_texture_is_up_to_date: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)layer {
    let new_layer: id = msg![env; this alloc];
    msg![env; new_layer init]
}

- (())dealloc {
    let &mut CALayerHostObject {
        drawable_properties,
        contents,
        superlayer,
        cg_context,
        ref mut sublayers,
        ..
    } = env.objc.borrow_mut(this);
    let sublayers = std::mem::take(sublayers);

    if drawable_properties != nil {
        release(env, drawable_properties);
    }

    if contents != nil {
        release(env, contents);
    }

    if let Some(cg_context) = cg_context {
        CGContextRelease(env, cg_context);
    }

    assert!(superlayer == nil);
    for sublayer in sublayers {
        env.objc.borrow_mut::<CALayerHostObject>(sublayer).superlayer = nil;
        release(env, sublayer);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)delegate {
    env.objc.borrow::<CALayerHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<CALayerHostObject>(this).delegate = delegate;
}

- (id)superlayer {
    env.objc.borrow::<CALayerHostObject>(this).superlayer
}
// TODO: sublayers accessors

- (())addSublayer:(id)layer {
    if env.objc.borrow::<CALayerHostObject>(layer).superlayer == this {
        () = msg![env; this bringSublayerToFront:layer];
    } else {
        retain(env, layer);
        () = msg![env; layer removeFromSuperlayer];
        env.objc.borrow_mut::<CALayerHostObject>(layer).superlayer = this;
        env.objc.borrow_mut::<CALayerHostObject>(this).sublayers.push(layer);
    }
}

- (())insertSublayer:(id)layer atIndex:(u32)idx {
    retain(env, layer);
    () = msg![env; layer removeFromSuperlayer];
    env.objc.borrow_mut::<CALayerHostObject>(layer).superlayer = this;

    let CALayerHostObject { ref mut sublayers, .. } = env.objc.borrow_mut(this);
    sublayers.insert(idx.try_into().unwrap(), layer);
}

- (())insertSublayer:(id)layer below:(id)sibling {
    retain(env, layer);
    () = msg![env; layer removeFromSuperlayer];
    env.objc.borrow_mut::<CALayerHostObject>(layer).superlayer = this;

    let CALayerHostObject { ref mut sublayers, .. } = env.objc.borrow_mut(this);
    let idx = sublayers.iter().position(|&sublayer| sublayer == sibling).unwrap();
    sublayers.insert(idx, layer);
}

- (())removeFromSuperlayer {
    let CALayerHostObject { ref mut superlayer, .. } = env.objc.borrow_mut(this);
    let superlayer = std::mem::take(superlayer);
    if superlayer == nil {
        return;
    }

    let CALayerHostObject { ref mut sublayers, .. } = env.objc.borrow_mut(superlayer);
    let idx = sublayers.iter().position(|&sublayer| sublayer == this).unwrap();
    let sublayer = sublayers.remove(idx);
    assert!(sublayer == this);
    release(env, this);
}

- (CGRect)bounds {
    env.objc.borrow::<CALayerHostObject>(this).bounds
}
- (())setBounds:(CGRect)bounds {
    env.objc.borrow_mut::<CALayerHostObject>(this).bounds = bounds;
}
- (CGPoint)position {
    env.objc.borrow::<CALayerHostObject>(this).position
}
- (())setPosition:(CGPoint)position {
    env.objc.borrow_mut::<CALayerHostObject>(this).position = position;
}
- (CGPoint)anchorPoint {
    env.objc.borrow::<CALayerHostObject>(this).anchor_point
}
- (())setAnchorPoint:(CGPoint)anchor_point {
    env.objc.borrow_mut::<CALayerHostObject>(this).anchor_point = anchor_point;
}
- (CGAffineTransform)affineTransform {
    env.objc.borrow::<CALayerHostObject>(this).affine_transform
}
- (())setAffineTransform:(CGAffineTransform)affine_transform {
    env.objc.borrow_mut::<CALayerHostObject>(this).affine_transform = affine_transform;
}

- (CGRect)frame {
    let host_obj @ &CALayerHostObject {
        bounds,
        ..
    } = env.objc.borrow(this);
    host_obj.superlayer_to_layer_transform().apply_to_rect(CGRect {
        origin: CGPoint { x: bounds.origin.x, y: bounds.origin.y },
        size: bounds.size,
    })
}
- (())setFrame:(CGRect)frame {
    let CALayerHostObject {
        bounds,
        position,
        anchor_point,
        affine_transform,
        ..
    } = env.objc.borrow_mut(this);

    let inverse_transform = CGAffineTransform::make_translation(
        -frame.size.width * anchor_point.x,
        -frame.size.height * anchor_point.y,
    )
    .concat(*affine_transform).invert();

    // Not the same as ::apply_to_size() as this does not ignore translation.
    let transformed_size = inverse_transform.apply_to_rect(CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: frame.size
    }).size;
    let transformed_offset = inverse_transform.apply_to_point(CGPoint { x: 0.0, y: 0.0 });
    *position = CGPoint {
        x: frame.origin.x + transformed_offset.x,
        y: frame.origin.y + transformed_offset.y,
    };
    *bounds = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: transformed_size,
    };
}

- (bool)isHidden {
    env.objc.borrow::<CALayerHostObject>(this).hidden
}
- (())setHidden:(bool)hidden {
    env.objc.borrow_mut::<CALayerHostObject>(this).hidden = hidden;
}

- (bool)isOpaque {
    env.objc.borrow::<CALayerHostObject>(this).opaque
}
- (())setOpaque:(bool)opaque {
    env.objc.borrow_mut::<CALayerHostObject>(this).opaque = opaque;
}

- (f32)opacity {
    env.objc.borrow::<CALayerHostObject>(this).opacity
}
- (())setOpacity:(f32)opacity {
    env.objc.borrow_mut::<CALayerHostObject>(this).opacity = opacity;
}

- (CGColorRef)backgroundColor {
    if let Some(bg_color) = env.objc.borrow::<CALayerHostObject>(this).background_color.clone() {
        let class = env.objc.get_known_class("_touchHLE_CGColor", &mut env.mem);
        let obj = env.objc.alloc_object(class, Box::new(bg_color), &mut env.mem);
        autorelease(env, obj)
    } else {
        nil
    }
}
- (())setBackgroundColor:(CGColorRef)new_color {
    let new_color = if new_color == nil {
        None
    } else {
        Some(env.objc.borrow::<CGColorHostObject>(new_color).clone())
    };
    env.objc.borrow_mut::<CALayerHostObject>(this).background_color = new_color;
}

- (CGFloat)cornerRadius {
    env.objc.borrow::<CALayerHostObject>(this).corner_radius
}
- (())setCornerRadius:(CGFloat)corner_radius {
    env.objc.borrow_mut::<CALayerHostObject>(this).corner_radius = corner_radius;
}

- (bool)needsDisplay {
    env.objc.borrow::<CALayerHostObject>(this).needs_display
}
- (())setNeedsDisplay {
    env.objc.borrow_mut::<CALayerHostObject>(this).needs_display = true;
}
// TODO: support setNeedsDisplayInRect:
- (())displayIfNeeded {
    let &mut CALayerHostObject {
        ref mut needs_display,
        delegate,
        ..
    } = env.objc.borrow_mut(this);
    if !std::mem::take(needs_display) {
        return;
    }

    if delegate == nil {
        return;
    }

    let delegate_class = ObjC::read_isa(delegate, &env.mem);

    // According to the Core Animation Programming Guide, a layer delegate must
    // provide either displayLayer: or drawLayer:inContext:, and the former is
    // called if both are defined.

    if env.objc.class_has_method_named(delegate_class, "displayLayer:") {
        () = msg![env; delegate displayLayer:this];
        return;
    }

    let &mut CALayerHostObject {
        cg_context,
        ref mut gles_texture_is_up_to_date,
        bounds: CGRect { origin, size },
        ..
    } = env.objc.borrow_mut(this);

    *gles_texture_is_up_to_date = false;

    // TODO: more correctly handle non-integer sizes?
    let int_width = size.width.round() as GuestUSize;
    let int_height = size.height.round() as GuestUSize;

    let need_new_context = cg_context.is_none_or(|existing|
            CGBitmapContextGetWidth(env, existing) != int_width ||
            CGBitmapContextGetHeight(env, existing) != int_height
    );
    let cg_context = if need_new_context {
        if let Some(old_context) = cg_context {
            CGContextRelease(env, old_context);
        }

        // Make sure this is in sync with the code in composition.rs that
        // uploads the texture!
        // TODO: is this the right color space?
        let color_space = CGColorSpaceCreateDeviceRGB(env);
        let cg_context = CGBitmapContextCreate(
            env,
            Ptr::null(),
            int_width,
            int_height,
            8, // bpp
            int_width.checked_mul(4).unwrap(),
            color_space,
            kCGImageByteOrder32Big | kCGImageAlphaPremultipliedLast
        );
        env.objc.borrow_mut::<CALayerHostObject>(this).cg_context = Some(cg_context);
        cg_context
    } else {
        cg_context.unwrap()
    };

    CGContextTranslateCTM(env, cg_context, -origin.x, -origin.y);
    // TODO: move clearing to UIKit (clearsContextBeforeDrawing)?
    CGContextClearRect(env, cg_context, CGRect { origin, size });
    () = msg![env; delegate drawLayer:this inContext:cg_context];
    CGContextTranslateCTM(env, cg_context, origin.x, origin.y);
}

// CGImageRef*
- (id)contents {
    env.objc.borrow::<CALayerHostObject>(this).contents
}
- (())setContents:(id)new_contents {
    let host_obj = env.objc.borrow_mut::<CALayerHostObject>(this);
    host_obj.gles_texture_is_up_to_date = false;
    let old_contents = std::mem::replace(&mut host_obj.contents, new_contents);
    retain(env, new_contents);
    release(env, old_contents);
}

- (())setEdgeAntialiasingMask:(u32)mask {
    log!("TODO: [(CALayer*){:?} setEdgeAntialiasingMask: {}]", this, mask); // TODO
}

- (())setMagnificationFilter:(id)filter {
    log!("TODO: [(CALayer*){:?} setMagnificationFilter: {}]", this, ns_string::to_rust_string(env, filter)); // TODO
}

- (())setMinificationFilter:(id)filter {
    log!("TODO: [(CALayer*){:?} setMinificationFilter: {}]", this, ns_string::to_rust_string(env, filter)); // TODO
}

- (bool)containsPoint:(CGPoint)point {
    let bounds: CGRect = msg![env; this bounds];
    let x_range = bounds.origin.x..(bounds.origin.x + bounds.size.width);
    let y_range = bounds.origin.y..(bounds.origin.y + bounds.size.height);
    let CGPoint {x, y} = point;
    x_range.contains(&x) && y_range.contains(&y)
}

- (CGPoint)convertPoint:(CGPoint)point
              fromLayer:(id)other { // CALayer*

    if this == other {
        return point;
    }

    let res = transform_for_conversion(env, this, other).apply_to_point(point);
    log_dbg!("Converted {point:?} from {other:?} to {this:?}: {res:?}");
    res
}
- (CGPoint)convertPoint:(CGPoint)point
                toLayer:(id)other { // CALayer*
    if this == other {
        return point;
    }

    let res = transform_for_conversion(env, other, this).apply_to_point(point);
    log_dbg!("Converted {point:?} from {this:?} to {other:?}: {res:?}");
    res
}
- (CGRect)convertRect:(CGRect)rect
            fromLayer:(id)other { // CALayer*

    if this == other {
        return rect;
    }

    let res = transform_for_conversion(env, this, other).apply_to_rect(rect);
    log_dbg!("Converted {rect:?} from {other:?} to {this:?}: {res:?}");
    res
}
- (CGRect)convertRect:(CGRect)rect
              toLayer:(id)other { // CALayer*
    if this == other {
        return rect;
    }

    let res = transform_for_conversion(env, other, this).apply_to_rect(rect);
    log_dbg!("Converted {rect:?} from {this:?} to {other:?}: {res:?}");
    res
}

// TODO: more

@end

};

fn transform_for_conversion(env: &mut Environment, this: id, other: id) -> CGAffineTransform {
    // The convertPoint methods can be used in two ways:
    // - If two layers are provided (one as the receiver, one as a parameter),
    //   then the layers are required to have a common ancestor, and it will be
    //   used to provide a reference for converting the point/rect.
    // - If one layer is provided, and the other layer is nil, then the layer
    //   is resolved to the co-ordinate space of the origin of the layer at the
    //   top of the hierarchy. This is effectively the same as screen space, or
    //   the co-ordinate space that windows live in.
    let need_common_ancestor = this != nil && other != nil;
    assert!(!(this == nil && other == nil));

    // This algorithm attempts to efficiently find the common ancestor of the
    // two layers by walking up each layer's superlayer chain, one at a time,
    // alternating between layers until it finds a match.
    // For the single-layer case, it of course only walks its superlayer chain.

    // Maps of layer pointers to transforms that map that layer's co-ordinate
    // space to that of the starting layer for the iteration.
    let mut this_map = HashMap::from([(this, CGAffineTransformIdentity)]);
    let mut other_map = HashMap::from([(other, CGAffineTransformIdentity)]);
    // Current iteration state.
    let mut this_superlayer = this;
    let mut this_transform = CGAffineTransformIdentity;
    let mut other_superlayer = other;
    let mut other_transform = CGAffineTransformIdentity;
    let (common_ancestor, this_transform, other_transform) = loop {
        if this_superlayer != nil {
            let this_hostobj: &CALayerHostObject = env.objc.borrow(this_superlayer);
            let next = this_hostobj.superlayer;
            let next_transform =
                this_transform.concat(this_hostobj.superlayer_to_layer_transform());
            if need_common_ancestor && next != nil {
                if let Some(&other_transform) = other_map.get(&next) {
                    break (next, next_transform, other_transform);
                }
                this_map.insert(next, next_transform);
            }
            this_superlayer = next;
            this_transform = next_transform;
        }

        if other_superlayer != nil {
            let other_hostobj: &CALayerHostObject = env.objc.borrow(other_superlayer);
            let next = other_hostobj.superlayer;
            let next_transform =
                other_transform.concat(other_hostobj.superlayer_to_layer_transform());
            if need_common_ancestor && next != nil {
                if let Some(&this_transform) = this_map.get(&next) {
                    break (next, this_transform, next_transform);
                }
                other_map.insert(next, next_transform);
            }
            other_superlayer = next;
            other_transform = next_transform;
        }

        if this_superlayer == nil && other_superlayer == nil {
            if need_common_ancestor {
                panic!("Layers {this:?} and {other:?} have no common ancestor!");
            } else {
                break (nil, this_transform, other_transform);
            }
        }
    };

    assert!((common_ancestor == nil) != need_common_ancestor);
    if need_common_ancestor {
        log_dbg!("{this:?} and {other:?}'s common ancestor: {common_ancestor:?}",);
    }
    log_dbg!("{this:?}'s transform in {common_ancestor:?}: {this_transform:?}");
    log_dbg!("{other:?}'s transform in {common_ancestor:?}: {other_transform:?}");
    let other_to_this = other_transform.concat(this_transform.invert());
    log_dbg!("Transform from {other:?} to {this:?}: {other_to_this:?}");
    other_to_this
}
