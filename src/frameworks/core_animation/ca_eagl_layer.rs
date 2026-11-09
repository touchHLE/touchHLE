/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAEAGLLayer`.

use super::ca_layer::CALayerHostObject;
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::objc::{id, msg, msg_class, nil, objc_classes, Class, ClassExports};
use crate::Environment;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CAEAGLLayer: CALayer

// EAGLDrawable implementation (the only one)

- (id)drawableProperties {
    // FIXME: do we need to return an empty dictionary rather than nil?
    env.objc.borrow::<CALayerHostObject>(this).drawable_properties
}

- (())setDrawableProperties:(id)props { // NSDictionary<NSString*, id>*
    let props: id = msg![env; props copy];
    env.objc.borrow_mut::<CALayerHostObject>(this).drawable_properties = props;
}

@end

};

/// If there is an opaque `CAEAGLLayer` that covers the entire screen, this
/// returns a pointer to it. Otherwise, it returns [nil].
///
/// To avoid a state management nightmare, we want to have an internal OpenGL ES
/// context for compositing, separate from any OpenGL ES contexts the app uses
/// for its rendering. When we have a `CAEAGLLayer` though, we need to transfer
/// a rendered frame from the app's context to the compositor's context, and
/// unfortunately the most practical way to do this is `glReadPixels()`, which
/// is highly inefficient. To make things efficient, then, we have a shortcut:
/// if the result of composition would be identical to the rendered frame, i.e.
/// there's a single full-screen layer, we skip transferring between contexts
/// and present it directly from the app's context. This function is used to
/// determine when that will happen.
/// Reports why a layer was rejected as the full-screen EAGL layer, then
/// returns [nil]. Presentation falls back to a slow path when this happens, so
/// the reason is worth having when diagnosing stutter.
fn reject(reason: &str, layer: id) -> id {
    log_dbg!("layer {:?} is not a full-screen EAGL layer: {}", layer, reason);
    nil
}

pub fn find_fullscreen_eagl_layer(env: &mut Environment) -> id {
    if env.options.force_composition {
        return nil;
    }

    let windows = env.framework_state.uikit.ui_view.ui_window.windows.clone();
    // Assumes the windows in the list are ordered back-to-front.
    // TODO: this may not be correct once we support windowLevel.
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

    // Descend through the hierarchy, looking only at the last layer in each
    // list of children, since that should be the one on top.
    // TODO: this is not correct once we support zPosition.
    loop {
        assert!(layer != nil);

        let layer_host_obj: &CALayerHostObject = env.objc.borrow(layer);

        // This is stricter than it should be. In theory we should accumulate
        // the transforms and handle different anchor points etc, but real apps
        // probably only use this common case.
        //
        // A layer does not have to be exactly as tall as the screen: apps with
        // a visible status bar get a shorter window, and MikuFlick/02's
        // EAGL layer is 320x460 on a 320x480 screen. Requiring an exact match
        // made presentation fall back to a glReadPixels() round-trip through
        // system memory on every frame, which stalls on the GPU sync point and
        // could show a black frame while the drawable was recycled. Covering
        // the full width from the top-left corner is what makes a layer able
        // to stand in for the whole frame.
        let bounds = layer_host_obj.bounds;
        let covers_full_width = bounds.origin == (CGPoint { x: 0.0, y: 0.0 })
            && bounds.size.width == screen_bounds.size.width
            && bounds.size.height > 0.0
            && bounds.size.height <= screen_bounds.size.height;
        // Layers are normally centred in their own bounds, and sometimes in the
        // screen bounds; both describe the same un-offset layer here.
        let centred_in_own_bounds = layer_host_obj.position
            == (CGPoint {
                x: bounds.size.width / 2.0,
                y: bounds.size.height / 2.0,
            });
        let centred_in_screen = layer_host_obj.position
            == (CGPoint {
                x: screen_bounds.size.width / 2.0,
                y: screen_bounds.size.height / 2.0,
            });
        let mismatch = format!(
            "bounds {:?}/{:?} vs screen {:?}/{:?}",
            bounds.origin, bounds.size, screen_bounds.origin, screen_bounds.size
        );
        let geometry_problem = if !covers_full_width {
            Some(mismatch.as_str())
        } else if layer_host_obj.anchor_point != (CGPoint { x: 0.5, y: 0.5 }) {
            Some("anchor point is not centred")
        } else if !centred_in_own_bounds && !centred_in_screen {
            Some("position is not centred")
        } else if layer_host_obj.hidden {
            Some("layer is hidden")
        } else if layer_host_obj.opacity != 1.0 {
            Some("layer is not fully opaque")
        } else if !layer_host_obj.affine_transform.is_identity() {
            // TODO: support affine transforms that result in a full-screen
            //       layer (typical example is 90° rotation).
            Some("layer has a non-identity transform")
        } else {
            None
        };
        if std::env::var_os("PEAR_LAYER_TREE").is_some() {
            log_dbg!(
                "layer {:?} bounds {:?}/{:?} opacity {} hidden {} opaque {} sublayers {}",
                layer,
                bounds.origin,
                bounds.size,
                layer_host_obj.opacity,
                layer_host_obj.hidden,
                layer_host_obj.opaque,
                layer_host_obj.sublayers.len(),
            );
        }
        if let Some(reason) = geometry_problem {
            return reject(reason, layer);
        }

        // Only the topmost *visible* sublayer can hide this one. Hidden layers
        // and fully transparent ones contribute nothing to composition, so
        // they must not block the fast path (MikuFlick/02 keeps a hidden 20x20
        // sublayer above its EAGL layer).
        let next = layer_host_obj
            .sublayers
            .iter()
            .rev()
            .copied()
            .find(|&sublayer| {
                let sublayer_host_obj: &CALayerHostObject = env.objc.borrow(sublayer);
                !sublayer_host_obj.hidden && sublayer_host_obj.opacity > 0.0
            });
        match next {
            Some(next) => layer = next,
            None => break,
        }
    }

    if !env.objc.borrow::<CALayerHostObject>(layer).opaque {
        return reject("layer is not opaque", layer);
    }

    let ca_eagl_layer_class: Class = msg_class![env; CAEAGLLayer class];
    if !msg![env; layer isKindOfClass:ca_eagl_layer_class] {
        return reject("layer is not a CAEAGLLayer", layer);
    }

    log_once!("Found a full-screen EAGL layer, presentation uses the fast path.");

    layer
}

/// For use by `EAGLContext` when presenting to a `CAEAGLLayer`:
/// [std::mem::take]s the buffer used to hold the pixels. It should be passed
/// back to [present_pixels] once it has been filled.
pub fn get_pixels_vec_for_presenting(env: &mut Environment, layer: id) -> Vec<u8> {
    env.objc
        .borrow_mut::<CALayerHostObject>(layer)
        .presented_pixels
        .take()
        .map(|(vec, _width, _height)| vec)
        .unwrap_or_default()
}

/// For use by `EAGLContext` when presenting to a `CAEAGLLayer`: provide the new
/// frame rendered by the app, so it can be used when compositing. The buffer
/// should have been obtained with [get_pixels_vec_for_presenting] before
/// filling. The data must be in RGBA8 format.
pub fn present_pixels(env: &mut Environment, layer: id, pixels: Vec<u8>, width: u32, height: u32) {
    let host_obj = env.objc.borrow_mut::<CALayerHostObject>(layer);
    host_obj.presented_pixels = Some((pixels, width, height));
    host_obj.gles_texture_is_up_to_date = false;
}
