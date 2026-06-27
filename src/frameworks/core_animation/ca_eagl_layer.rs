/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAEAGLLayer`.

use super::ca_layer::CALayerHostObject;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect};
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class, nil, objc_classes, release, Class, ClassExports};
use crate::Environment;

// MARK: - EAGLDrawable property key constants
//
// These are the keys apps put in the drawableProperties dictionary.
// We export them as static strings so other modules can reference them.
pub const kEAGLDrawablePropertyRetainedBacking: &str = "kEAGLDrawablePropertyRetainedBacking";
pub const kEAGLDrawablePropertyColorFormat: &str = "kEAGLDrawablePropertyColorFormat";

// kEAGLColorFormat values
pub const kEAGLColorFormatRGBA8: &str = "kEAGLColorFormatRGBA8";
pub const kEAGLColorFormatRGB565: &str = "kEAGLColorFormatRGB565";
pub const kEAGLColorFormatSRGBA8: &str = "kEAGLColorFormatSRGBA8";

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CAEAGLLayer: CALayer

// =========================================================================
// MARK: - EAGLDrawable protocol
// =========================================================================

- (id)drawableProperties { // NSDictionary<NSString*, id>*
    env.objc.borrow::<CALayerHostObject>(this).drawable_properties
}

- (())setDrawableProperties:(id)props { // NSDictionary<NSString*, id>*
    // Store a copy — matches Apple's behaviour.
    let old = env.objc.borrow::<CALayerHostObject>(this).drawable_properties;
    let new_props: id = if props != nil { msg![env; props copy] } else { nil };
    release(env, old);
    env.objc.borrow_mut::<CALayerHostObject>(this).drawable_properties = new_props;

    // Log the properties the app is requesting so rendering issues are
    // easier to diagnose.
    if new_props != nil {
        let retained_key = ns_string::get_static_str(env, kEAGLDrawablePropertyRetainedBacking);
        let format_key    = ns_string::get_static_str(env, kEAGLDrawablePropertyColorFormat);

        let retained_val: id = msg![env; new_props objectForKey:retained_key];
        let format_val:   id = msg![env; new_props objectForKey:format_key];

        let retained_str = if retained_val != nil {
            let b: bool = msg![env; retained_val boolValue];
            if b { "YES" } else { "NO" }
        } else {
            "(not set)"
        };

        let format_str = if format_val != nil {
            ns_string::to_rust_string(env, format_val).into_owned()
        } else {
            "(not set)".to_string()
        };

        log_dbg!(
            "CAEAGLLayer setDrawableProperties: retainedBacking={} colorFormat={}",
            retained_str, format_str
        );
    }
}

// =========================================================================
// MARK: - Convenience helpers (read individual drawable properties)
// =========================================================================

// Returns YES if the backing store should be retained after presentation.
// Corresponds to kEAGLDrawablePropertyRetainedBacking.
- (bool)_touchHLE_retainedBacking {
    let props = env.objc.borrow::<CALayerHostObject>(this).drawable_properties;
    if props == nil { return false; }
    let key: id = ns_string::get_static_str(env, kEAGLDrawablePropertyRetainedBacking);
    let val: id = msg![env; props objectForKey:key];
    if val == nil { return false; }
    msg![env; val boolValue]
}

// Returns the requested color format string, or "kEAGLColorFormatRGBA8"
// as the default if not specified.
- (id)_touchHLE_colorFormat { // NSString*
    let props = env.objc.borrow::<CALayerHostObject>(this).drawable_properties;
    if props != nil {
        let key: id = ns_string::get_static_str(env, kEAGLDrawablePropertyColorFormat);
        let val: id = msg![env; props objectForKey:key];
        if val != nil { return val; }
    }
    ns_string::get_static_str(env, kEAGLColorFormatRGBA8)
}

// =========================================================================
// MARK: - CALayer overrides
// =========================================================================

// CAEAGLLayer is always opaque by default (unlike plain CALayer).
- (id)init {
    let _: () = msg![env; this setOpaque:true];
    this
}

// MARK: - Scale / Retina Support

- (CGFloat)contentScaleFactor {
    // Жестко задаем масштаб 1.0 (стандартный не-Retina экран)
    1.0
}

- (())setContentScaleFactor:(CGFloat)scale {
    // Заглушка, чтобы игра не упала, если попытается сама установить масштаб
    log_dbg!("CAEAGLLAYER setContentScaleFactor: {} (stubbed)", scale);
}

- (CGFloat)contentsScale {
    // Жестко задаем масштаб 1.0 (стандартный не-Retina экран)
    1.0
}

- (())setContentsScale:(CGFloat)scale {
    // Заглушка, чтобы игра не упала, если попытается сама установить масштаб
    log_dbg!("CAEAGLLAYER setContentsScale: {} (stubbed)", scale);
}

- (id)initWithLayer:(id)layer {
    let _: () = msg![env; this setOpaque:true];
    // Copy drawable properties from the source layer if it is also a
    // CAEAGLLayer.
    let ca_eagl_class: Class = msg_class![env; CAEAGLLayer class];
    let is_eagl: bool = msg![env; layer isKindOfClass:ca_eagl_class];
    if is_eagl {
        let src_props = env.objc.borrow::<CALayerHostObject>(layer).drawable_properties;
        if src_props != nil {
            let copy: id = msg![env; src_props copy];
            env.objc.borrow_mut::<CALayerHostObject>(this).drawable_properties = copy;
        }
    }
    this
}

// Prevent the layer from being drawn by Core Animation — its contents are
// managed exclusively by EAGL/OpenGL ES.
- (())display {
    // No-op: the layer is presented via EAGLContext presentRenderBuffer:.
}

- (())drawInContext:(id)_ctx { // CGContextRef
    // No-op: CAEAGLLayer content comes from OpenGL ES, not Core Graphics.
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let host = env.objc.borrow::<CALayerHostObject>(this);
    let opaque = host.opaque;
    let has_props = host.drawable_properties != nil;
    let s = format!(
        "<CAEAGLLayer: {:?}; opaque={}; drawableProperties={}>",
        this,
        opaque,
        if has_props { "(set)" } else { "(nil)" }
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

// =========================================================================
// MARK: - find_fullscreen_eagl_layer
// =========================================================================

/// If there is an opaque `CAEAGLLayer` that covers the entire screen, this
/// returns a pointer to it. Otherwise, it returns [nil].
pub fn find_fullscreen_eagl_layer(env: &mut Environment) -> id {
    if env.options.force_composition {
        return nil;
    }

    let windows = env.framework_state.uikit.ui_view.ui_window.windows.clone();
    let Some(top_window) = windows
        .into_iter()
        .rev()
        .find(|&window| !msg![env; window isHidden])
    else {
        return nil;
    };

    let screen_bounds: CGRect = {
        let screen: id = msg_class![env; UIScreen mainScreen];
        msg![env; screen bounds]
    };

    let mut layer: id = msg![env; top_window layer];

    loop {
        // assert!(layer != nil);

        let layer_host_obj: &CALayerHostObject = env.objc.borrow(layer);

        if layer_host_obj.bounds.size != screen_bounds.size
            || layer_host_obj.bounds.origin != (CGPoint { x: 0.0, y: 0.0 })
            || layer_host_obj.anchor_point != (CGPoint { x: 0.5, y: 0.5 })
            || layer_host_obj.position
                != (CGPoint {
                    x: screen_bounds.size.width / 2.0,
                    y: screen_bounds.size.height / 2.0,
                })
            || layer_host_obj.hidden
            || layer_host_obj.opacity != 1.0
            || !layer_host_obj.affine_transform.is_identity()
        {
            return nil;
        }

        if let Some(&next) = layer_host_obj.sublayers.last() {
            layer = next;
        } else {
            break;
        }
    }

    if !env.objc.borrow::<CALayerHostObject>(layer).opaque {
        return nil;
    }

    let ca_eagl_layer_class: Class = msg_class![env; CAEAGLLayer class];
    if !msg![env; layer isKindOfClass:ca_eagl_layer_class] {
        return nil;
    }

    layer
}

// =========================================================================
// MARK: - Pixel buffer helpers (used by EAGLContext)
// =========================================================================

/// Takes the pixel buffer out of the layer so it can be refilled by
/// `EAGLContext presentRenderBuffer:`. Pass the buffer back via
/// [present_pixels] once it is filled.
pub fn get_pixels_vec_for_presenting(env: &mut Environment, layer: id) -> Vec<u8> {
    env.objc
        .borrow_mut::<CALayerHostObject>(layer)
        .presented_pixels
        .take()
        .map(|(vec, _w, _h)| vec)
        .unwrap_or_default()
}

/// Stores the new rendered frame in the layer and marks the GLES texture as
/// stale. Data must be in RGBA8 format.
pub fn present_pixels(env: &mut Environment, layer: id, pixels: Vec<u8>, width: u32, height: u32) {
    let host_obj = env.objc.borrow_mut::<CALayerHostObject>(layer);
    host_obj.presented_pixels = Some((pixels, width, height));
    host_obj.gles_texture_is_up_to_date = false;
}

/// Returns whether the layer's backing store should be retained after
/// presentation (i.e. `kEAGLDrawablePropertyRetainedBacking` is YES).
/// Convenience wrapper for use by `EAGLContext`.
pub fn is_retained_backing(env: &mut Environment, layer: id) -> bool {
    msg![env; layer _touchHLE_retainedBacking]
}

/// Returns the drawable's pixel width (from the presented pixel buffer if
/// available, otherwise from the layer's bounds).
pub fn drawable_width(env: &mut Environment, layer: id) -> u32 {
    let host = env.objc.borrow::<CALayerHostObject>(layer);
    if let Some((_, w, _)) = host.presented_pixels {
        return w;
    }
    host.bounds.size.width as u32
}

/// Returns the drawable's pixel height.
pub fn drawable_height(env: &mut Environment, layer: id) -> u32 {
    let host = env.objc.borrow::<CALayerHostObject>(layer);
    if let Some((_, _, h)) = host.presented_pixels {
        return h;
    }
    host.bounds.size.height as u32
}
