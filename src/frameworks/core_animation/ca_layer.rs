/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CALayer`.

use crate::frameworks::core_foundation::{CFRelease, CFRetain};
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextCreate, CGBitmapContextGetHeight, CGBitmapContextGetWidth,
};
use crate::frameworks::core_graphics::cg_color_space::CGColorSpaceCreateDeviceRGB;
use crate::frameworks::core_graphics::cg_context::{
    CGContextClearRect, CGContextRef, CGContextRelease, CGContextTranslateCTM,
};
use crate::frameworks::core_graphics::cg_image::{
    kCGImageAlphaPremultipliedLast, kCGImageByteOrder32Big,
};
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::mem::{GuestUSize, Ptr};
use crate::objc::{id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, ObjC};
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
    pub(super) hidden: bool,
    pub(super) opaque: bool,
    pub(super) opacity: f32,
    pub(super) background_color: id,
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
        hidden: false,
        opaque: false,
        opacity: 1.0,
        background_color: nil, // transparency
        needs_display: true,
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
        background_color,
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

    if background_color != nil {
        CFRelease(env, background_color);
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

- (CGRect)frame {
    let &CALayerHostObject {
        bounds,
        position,
        anchor_point,
        ..
    } = env.objc.borrow(this);
    CGRect {
        origin: CGPoint {
            x: position.x - bounds.size.width * anchor_point.x,
            y: position.y - bounds.size.height * anchor_point.y,
        },
        size: bounds.size,
    }
}
- (())setFrame:(CGRect)frame {
    let CALayerHostObject {
        bounds,
        position,
        anchor_point,
        ..
    } = env.objc.borrow_mut(this);
    *position = CGPoint {
        x: frame.origin.x + frame.size.width * anchor_point.x,
        y: frame.origin.y + frame.size.height * anchor_point.y,
    };
    *bounds = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: frame.size,
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

// See remarks in ui_view.rs about the type of this property
- (id)backgroundColor {
    env.objc.borrow::<CALayerHostObject>(this).background_color
}
- (())setBackgroundColor:(id)new_color {
    let host_obj = env.objc.borrow_mut::<CALayerHostObject>(this);
    let old_color = std::mem::replace(&mut host_obj.background_color, new_color);
    if new_color != nil {
        CFRetain(env, new_color); // CFRetain doesn't like nil
    }
    if old_color != nil {
        CFRelease(env, old_color); // CFRelease doesn't like nil
    }
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

    // UIView has a method called drawRect: that subclasses override if they
    // need custom drawing. touchHLE's UIView (a CALayerDelegate) provides
    // an implementation of drawLayer:inContext: that calls drawRect:.
    // This maintains a clean separation of UIView and CALayer, but it also
    // means that CALayer has no idea which views actually need custom drawing,
    // because they all have the inherited drawLayer:inContext: method.
    // To avoid wasting space and time on unnecessary bitmaps, let's pierce the
    // veil.
    // (TODO: somehow do this optimization in UIView rather than CALayer.
    // Apparently Apple do it that way: https://stackoverflow.com/q/4979192)
    let ui_view_class = env.objc.get_known_class("UIView", &mut env.mem);
    if env.objc.class_is_subclass_of(delegate_class, ui_view_class) {
        let draw_rect_sel = env.objc.lookup_selector("drawRect:").unwrap();
        let draw_layer_sel = env.objc.lookup_selector("drawLayer:inContext:").unwrap();
        if !env.objc.class_overrides_method_of_superclass(
            delegate_class,
            draw_rect_sel,
            ui_view_class
        ) && !env.objc.class_overrides_method_of_superclass(
            delegate_class,
            draw_layer_sel,
            ui_view_class
        ) {
            log_dbg!("Skipped render! {:?} does not override UIView's drawRect: or drawLayer:inContext: methods.", delegate_class);
            return;
        }
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

    let need_new_context = cg_context.map_or(
        true,
        |existing| (
            CGBitmapContextGetWidth(env, existing) != int_width ||
            CGBitmapContextGetHeight(env, existing) != int_height
        )
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

- (bool)containsPoint:(CGPoint)point {
    let bounds: CGRect = msg![env; this bounds];
    let x_range = bounds.origin.x..(bounds.origin.x + bounds.size.width);
    let y_range = bounds.origin.y..(bounds.origin.y + bounds.size.height);
    let CGPoint {x, y} = point;
    x_range.contains(&x) && y_range.contains(&y)
}

- (CGPoint)convertPoint:(CGPoint)point
              fromLayer:(id)other { // CALayer*
    assert!(other != nil); // TODO

    if this == other {
        return point;
    }

    // The two layers must have a common ancestor. Let's try to find it.
    // The idea is to walk up each layer's superlayer chain, one at a time,
    // alternating between layers until we find a match.

    // Maps of layer pointers to origins in that layer's co-ordinate space.
    let mut this_map = HashMap::from([(this, CGPoint { x: 0.0, y: 0.0 })]);
    let mut other_map = HashMap::from([(other, CGPoint { x: 0.0, y: 0.0 })]);
    // Current iteration state.
    let mut this_superlayer = this;
    let mut this_origin = CGPoint { x: 0.0, y: 0.0 };
    let mut other_superlayer = other;
    let mut other_origin = CGPoint { x: 0.0, y: 0.0 };
    let (common_ancestor, this_origin, other_origin) = loop {
        if this_superlayer != nil {
            let next: id = msg![env; this_superlayer superlayer];
            if next == nil {
                this_superlayer = nil;
            } else {
                let bounds: CGRect = msg![env; this_superlayer bounds];
                let frame: CGRect = msg![env; this_superlayer frame];
                let next_origin = CGPoint {
                    x: this_origin.x + frame.origin.x - bounds.origin.x,
                    y: this_origin.y + frame.origin.y - bounds.origin.y,
                };
                if let Some(&other_origin) = other_map.get(&next) {
                    break (next, next_origin, other_origin);
                }
                this_map.insert(next, next_origin);
                this_superlayer = next;
                this_origin = next_origin;
            }
        }

        if other_superlayer != nil {
            let next: id = msg![env; other_superlayer superlayer];
            if next == nil {
                other_superlayer = nil;
            } else {
                let bounds: CGRect = msg![env; other_superlayer bounds];
                let frame: CGRect = msg![env; other_superlayer frame];
                let next_origin = CGPoint {
                    x: other_origin.x + frame.origin.x - bounds.origin.x,
                    y: other_origin.y + frame.origin.y - bounds.origin.y,
                };
                if let Some(&this_origin) = this_map.get(&next) {
                    break (next, this_origin, next_origin);
                }
                other_map.insert(next, next_origin);
                other_superlayer = next;
                other_origin = next_origin;
            }
        }

        assert!(
            this_superlayer != nil || other_superlayer != nil,
            "Layers have no common ancestor!"
        );
    };

    log_dbg!("{:?} and {:?}'s common ancestor: {:?}", this, other, common_ancestor);
    log_dbg!("{:?}'s origin in common ancestor: {:?}", this, this_origin);
    log_dbg!("{:?}'s origin in common ancestor: {:?}", other, other_origin);
    let res = CGPoint {
        x: point.x + other_origin.x - this_origin.x,
        y: point.y + other_origin.y - this_origin.y,
    };
    log_dbg!("Converted {:?} from {:?} to {:?}: {:?}", point, other, this, res);
    res
}

- (CGPoint)convertPoint:(CGPoint)point
                toLayer:(id)other { // CALayer*
    assert!(other != nil); // TODO

    msg![env; other convertPoint:point fromLayer:this]
}

// TODO: more

@end

};
